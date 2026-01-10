use web_sys::MouseEvent;
use yew::{classes, function_component, html, Callback, Html, NodeRef, Properties};
use crate::app::components::AppIcon;

#[derive(Properties, Clone, PartialEq, Debug)]
pub struct IconButtonProps {
    pub name: String,
    pub icon: String,
    pub onclick: Callback<(String, MouseEvent)>,
    #[prop_or_default]
    pub class: String,
    #[prop_or_default]
    pub button_ref: Option<NodeRef>,
    #[prop_or_default]
    pub aria_label: Option<String>,
    #[prop_or_default]
    pub title: Option<String>,
}

#[function_component]
pub fn IconButton(props: &IconButtonProps) -> Html {

    let handle_click = {
        let click = props.onclick.clone();
        let name = props.name.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            click.emit((name.clone(), e));
        })
    };

    let aria_label = props.aria_label.clone().unwrap_or(props.name.clone());
    let title = props.title.clone().unwrap_or(aria_label.clone());

    html! {
        <button
            ref={props.button_ref.clone().unwrap_or_default()}
            class={classes!("tp__icon-button", props.class.clone())}
            onclick={handle_click}
            aria-label={aria_label}
            title={title}
        >
            <AppIcon name={props.icon.clone()}></AppIcon>
        </button>
    }
}
