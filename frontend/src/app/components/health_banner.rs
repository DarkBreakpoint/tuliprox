use crate::{
    app::context::{ConfigContext, StatusContext},
    hooks::use_service_context,
    i18n::use_translation,
    model::EventMessage,
    utils::set_location_hash,
};
use crate::model::ViewType;
use std::collections::HashMap;
use yew::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Health {
    Unknown,
    Healthy,
    Degraded,
    Unhealthy,
}

impl Health {
    fn modifier(self) -> &'static str {
        match self {
            Health::Unknown => "tp__health-banner--unknown",
            Health::Healthy => "tp__health-banner--healthy",
            Health::Degraded => "tp__health-banner--degraded",
            Health::Unhealthy => "tp__health-banner--unhealthy",
        }
    }
}


#[derive(Clone, Copy, PartialEq, Eq)]
enum Signal {
    Ok,
    Warn,
    Bad,
}

impl Signal {
    fn modifier(self) -> &'static str {
        match self {
            Signal::Ok => "tp__health-banner__signal--ok",
            Signal::Warn => "tp__health-banner__signal--warn",
            Signal::Bad => "tp__health-banner__signal--bad",
        }
    }
}

// A provider is considered "near capacity" (amber) at or above this usage ratio.
const CAPACITY_WARN_RATIO: f64 = 0.8;

struct ProviderRow {
    name: String,
    current: usize,
    max: u16,
    signal: Signal,
}

fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

#[component]
pub fn HealthBanner() -> Html {
    let services = use_service_context();
    let translate = use_translation();
    let status_ctx = use_context::<StatusContext>();
    let config_ctx = use_context::<ConfigContext>();
    let banner_ref = use_node_ref();
    let popover_pos = use_state(|| (0.0_f64, 0.0_f64));

    let ws_connected = use_state(|| true);
    {
        let ws_connected = ws_connected.clone();
        let services = services.clone();
        use_effect_with((), move |_| {
            let services_ctx = services.clone();
            let ws_connected = ws_connected.clone();
            let subid = services_ctx.event.subscribe(move |msg| {
                if let EventMessage::WebSocketStatus(active) = msg {
                    ws_connected.set(active);
                }
            });
            move || services_ctx.event.unsubscribe(subid)
        });
    }

    let status = status_ctx.as_ref().and_then(|ctx| ctx.status.clone());
    let has_status = status.is_some();
    let ws_ok = *ws_connected;

    let max_lookup: HashMap<String, u16> = config_ctx
        .as_ref()
        .and_then(|ctx| ctx.config.as_ref())
        .map(|cfg| {
            let mut map = HashMap::new();
            for input in &cfg.sources.inputs {
                map.insert(input.name.to_string(), input.max_connections);
                if let Some(aliases) = &input.aliases {
                    for alias in aliases {
                        map.insert(alias.name.to_string(), alias.max_connections);
                    }
                }
            }
            map
        })
        .unwrap_or_default();

    let mut provider_rows: Vec<ProviderRow> = Vec::new();
    let mut worst_ratio = 0.0_f64;
    let mut saturated = false;
    let backend_ok = match &status {
        Some(stats) => {
            if let Some(map) = &stats.active_provider_connections {
                for (name, current) in map {
                    let max = max_lookup.get(name.as_ref()).copied().unwrap_or(0);
                    if *current == 0 && max == 0 {
                        continue;
                    }
                    let signal = if max == 0 {
                        Signal::Ok
                    } else {
                        let ratio = *current as f64 / f64::from(max);
                        if ratio > worst_ratio {
                            worst_ratio = ratio;
                        }
                        if ratio >= 1.0 {
                            saturated = true;
                            Signal::Bad
                        } else if ratio >= CAPACITY_WARN_RATIO {
                            Signal::Warn
                        } else {
                            Signal::Ok
                        }
                    };
                    provider_rows.push(ProviderRow { name: name.to_string(), current: *current, max, signal });
                }
            }
            stats.status == "ok"
        }
        None => true,
    };

    let health = if !ws_ok {
        Health::Unhealthy
    } else if !has_status {
        Health::Unknown
    } else if !backend_ok || saturated {
        Health::Unhealthy
    } else if worst_ratio >= CAPACITY_WARN_RATIO {
        Health::Degraded
    } else {
        Health::Healthy
    };

    let last_change = use_state(|| js_sys::Date::now());
    {
        let last_change = last_change.clone();
        use_effect_with(health, move |_| {
            last_change.set(js_sys::Date::now());
            || ()
        });
    }
    let elapsed_secs = ((js_sys::Date::now() - *last_change) / 1000.0).max(0.0) as u64;

    let label = match health {
        Health::Healthy => translate.t("LABEL.HEALTH_HEALTHY"),
        Health::Degraded => translate.t("LABEL.HEALTH_DEGRADED"),
        Health::Unhealthy => translate.t("LABEL.HEALTH_UNHEALTHY"),
        Health::Unknown => translate.t("LABEL.HEALTH_UNKNOWN"),
    };
    let aria_label = format!("{}: {label}", translate.t("LABEL.HEALTH_BANNER"));

    let onclick = Callback::from(|_| set_location_hash(ViewType::Stats.as_str()));
    let onkeydown = Callback::from(|e: KeyboardEvent| {
        if e.key() == "Enter" || e.key() == " " {
            e.prevent_default();
            set_location_hash(ViewType::Stats.as_str());
        }
    });

    let update_popover_pos = {
        let banner_ref = banner_ref.clone();
        let popover_pos = popover_pos.clone();
        Callback::from(move |_: ()| {
            if let Some(el) = banner_ref.cast::<web_sys::Element>() {
                let rect = el.get_bounding_client_rect();
                let viewport_w = web_sys::window()
                    .and_then(|w| w.inner_width().ok())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(rect.right());
                let top = rect.bottom() + 8.0;
                let right = (viewport_w - rect.right()).max(0.0);
                popover_pos.set((top, right));
            }
        })
    };
    let onmouseenter = {
        let update_popover_pos = update_popover_pos.clone();
        Callback::from(move |_: MouseEvent| update_popover_pos.emit(()))
    };
    let onfocus = {
        let update_popover_pos = update_popover_pos.clone();
        Callback::from(move |_: FocusEvent| update_popover_pos.emit(()))
    };
    let (popover_top, popover_right) = *popover_pos;
    let popover_style = format!("top:{popover_top:.0}px;right:{popover_right:.0}px");

    // Popover signal rows.
    let ws_signal = if ws_ok { Signal::Ok } else { Signal::Bad };
    let ws_value = translate.t(if ws_ok { "LABEL.HEALTH_CONNECTED" } else { "LABEL.HEALTH_DISCONNECTED" });

    let providers_summary = if provider_rows.is_empty() {
        translate.t("LABEL.HEALTH_PROVIDERS_NONE")
    } else {
        let warn_count = provider_rows.iter().filter(|p| p.signal != Signal::Ok).count();
        if warn_count == 0 {
            translate.t("LABEL.HEALTH_PROVIDERS_OK")
        } else {
            format!("{warn_count}/{}", provider_rows.len())
        }
    };

    let provider_detail = provider_rows
        .iter()
        .map(|row| {
            let ratio = if row.max == 0 { 0.0 } else { (row.current as f64 / f64::from(row.max)).clamp(0.0, 1.0) };
            let value = if row.max == 0 {
                format!("{} / ∞", row.current)
            } else {
                format!("{} / {}", row.current, row.max)
            };
            html! {
                <div class="tp__health-banner__provider">
                    <span class={classes!("tp__health-banner__signal", row.signal.modifier())} aria-hidden="true" />
                    <span class="tp__health-banner__provider-name">{ row.name.clone() }</span>
                    <span class="tp__health-banner__bar" aria-hidden="true">
                        <span class="tp__health-banner__bar-fill" style={format!("width:{:.0}%", ratio * 100.0)} />
                    </span>
                    <span class="tp__health-banner__provider-value">{ value }</span>
                </div>
            }
        })
        .collect::<Html>();

    let backend_row = has_status.then(|| {
        let backend_signal = if backend_ok { Signal::Ok } else { Signal::Bad };
        let backend_value = status.as_ref().map_or_else(|| "n/a".to_string(), |s| s.status.clone());
        html! {
            <div class="tp__health-banner__row">
                <span class={classes!("tp__health-banner__signal", backend_signal.modifier())} aria-hidden="true" />
                <span class="tp__health-banner__row-label">{ translate.t("LABEL.HEALTH_BACKEND") }</span>
                <span class="tp__health-banner__row-value">{ backend_value }</span>
            </div>
        }
    });

    html! {
        <div
            ref={banner_ref}
            class={classes!("tp__health-banner", health.modifier())}
            role="status"
            aria-live="polite"
            aria-label={aria_label}
            tabindex="0"
            onclick={onclick}
            onkeydown={onkeydown}
            onmouseenter={onmouseenter}
            onfocus={onfocus}
        >
            <span class="tp__health-banner__dot" aria-hidden="true" />
            <span class="tp__health-banner__label">{ label }</span>
            <div class="tp__health-banner__popover" role="presentation" style={popover_style}>
                <div class="tp__health-banner__popover-head">
                    <span class="tp__health-banner__popover-title">{ translate.t("LABEL.HEALTH_BANNER") }</span>
                    <span class="tp__health-banner__popover-since">{ format_elapsed(elapsed_secs) }</span>
                </div>
                <div class="tp__health-banner__row">
                    <span class={classes!("tp__health-banner__signal", ws_signal.modifier())} aria-hidden="true" />
                    <span class="tp__health-banner__row-label">{ translate.t("LABEL.HEALTH_WEBSOCKET") }</span>
                    <span class="tp__health-banner__row-value">{ ws_value }</span>
                </div>
                { backend_row.unwrap_or_default() }
                <div class="tp__health-banner__row">
                    <span class="tp__health-banner__row-label">{ translate.t("LABEL.HEALTH_PROVIDERS") }</span>
                    <span class="tp__health-banner__row-value">{ providers_summary }</span>
                </div>
                { provider_detail }
            </div>
        </div>
    }
}
