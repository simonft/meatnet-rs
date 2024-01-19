mod bluetooth;
mod chart;
mod history;
mod components;

use std::{collections::BTreeMap, panic};

use bluetooth::{get_characteristics_and_listeners_from_service, ConnectionState};
use leptos::*;

use leptos_use::{storage::{use_local_storage, JsonCodec}, watch_throttled};
use meatnet::uart::node::response::ReadLogs;
use thaw::{Button, ButtonVariant};
use uuid::Uuid;

use crate::{chart::chart_handler, history::request_log_updates};

const NODE_UART_UUID: Uuid = uuid::uuid!("6E400001-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_RX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400002-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_TX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400003-B5A3-F393-E0A9-E50E24DCCA9E");

#[component]
fn App() -> impl IntoView {
    let (history, set_history, reset_history) =
        use_local_storage::<BTreeMap<u32, ReadLogs>, JsonCodec>("history");

    let (state, set_state) = create_signal(ConnectionState::Disconnected);
    let (get_chart, set_chart) = create_signal(None);

    let get_characteristics_and_listeners = create_action(move |_| {

        async move {
            get_characteristics_and_listeners_from_service(NODE_UART_UUID, UART_RX_CHARACTERISTIC_UUID, UART_TX_CHARACTERISTIC_UUID, set_state, set_history)
                .await
        }
    });

    let leptos_use::UseIntervalReturn { counter, .. } = leptos_use::use_interval(1000);

    let _refresh_effect = create_effect(move |_| {
        counter.get();

        spawn_local(async move {
            request_log_updates(history, state, get_characteristics_and_listeners).await
        });
    });

    let _history_update = watch_throttled(move|| history.get(), 
    move |history, _, _| {
        chart_handler(history, set_chart, get_chart);
    }, 5000.0);

    request_idle_callback(move || {
        chart_handler(&history.get_untracked(), set_chart, get_chart);
    });

    view! {
        <div class="container mx-auto w-[1200px]">
            <div class="flex flex-row">
                <div class="basis-3/4">
                    <div id="chart" class="chart"></div>
                </div>

                <components::LiveTempContainer state=state></components::LiveTempContainer>
            </div>
            <div class="flex flex-row justify-center">

                <Button
                    class="mx-10"
                    variant=ButtonVariant::Primary
                    on:click=move |_| {
                        get_characteristics_and_listeners.dispatch(());
                    }
                >

                    "Connect"
                </Button>
                <Button
                    class="mx-10"
                    variant=ButtonVariant::Primary
                    style=""
                    on:click=move |_| {
                        reset_history();
                        set_history.set(BTreeMap::new());
                    }
                >

                    "Reset"
                </Button>
            </div>
        </div>
    }

}

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    leptos::mount_to_body(|| view! { <App/> })
}
