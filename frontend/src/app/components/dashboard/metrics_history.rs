use crate::app::components::StatusContext;
use shared::model::{StatusCheck, SystemInfo};
use std::{collections::VecDeque, rc::Rc};
use yew::prelude::*;

const HISTORY_LEN: usize = 40;

fn push_capped(buffer: &mut VecDeque<f64>, value: f64) {
    if buffer.len() == HISTORY_LEN {
        buffer.pop_front();
    }
    buffer.push_back(value);
}

/// Rolling time-series of the key server metrics, fed by the streaming status updates.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct MetricsHistory {
    pub cpu: VecDeque<f64>,
    pub memory: VecDeque<f64>,
    pub net_rx: VecDeque<f64>,
    pub net_tx: VecDeque<f64>,
    pub users: VecDeque<f64>,
    pub connections: VecDeque<f64>,
}

impl MetricsHistory {
    fn record(&mut self, info: &SystemInfo, status: Option<&StatusCheck>) {
        push_capped(&mut self.cpu, f64::from(info.cpu_usage));
        let mem_pct = if info.memory_total > 0 {
            (info.memory_usage as f64 / info.memory_total as f64) * 100.0
        } else {
            0.0
        };
        push_capped(&mut self.memory, mem_pct);
        push_capped(&mut self.net_rx, info.net_rx_bytes_per_sec);
        push_capped(&mut self.net_tx, info.net_tx_bytes_per_sec);
        if let Some(status) = status {
            push_capped(&mut self.users, status.active_users as f64);
            push_capped(&mut self.connections, status.active_user_connections as f64);
        }
    }

    pub fn as_vec(buffer: &VecDeque<f64>) -> Vec<f64> {
        buffer.iter().copied().collect()
    }
}


#[hook]
pub fn use_metrics_history() -> Rc<MetricsHistory> {
    let status_ctx = use_context::<StatusContext>().expect("Status context not found");
    let history = use_state(|| Rc::new(MetricsHistory::default()));

    {
        let history = history.clone();
        let status = status_ctx.status.clone();
        use_effect_with(status_ctx.system_info.clone(), move |system_info| {
            if let Some(info) = system_info.as_ref() {
                let mut next = (**history).clone();
                next.record(info, status.as_deref());
                history.set(Rc::new(next));
            }
            || ()
        });
    }

    (*history).clone()
}
