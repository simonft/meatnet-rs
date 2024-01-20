mod bluetooth;
mod chart;
mod history;
mod components;

use std::{collections::BTreeMap, panic};

use bluetooth::{get_characteristics_and_listeners_from_service, ConnectionState};
use gloo::timers::future::TimeoutFuture;
use leptos::*;

use leptos_use::{storage::{use_local_storage, JsonCodec}, watch_throttled};
use meatnet::uart::node::response::ReadLogs;
use thaw::{Button, ButtonVariant};
use uuid::Uuid;

use crate::{chart::chart_handler, history::request_log_updates, bluetooth::setup_disconnect_handler};

const NODE_UART_UUID: Uuid = uuid::uuid!("6E400001-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_RX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400002-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_TX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400003-B5A3-F393-E0A9-E50E24DCCA9E");

#[component]
fn App() -> impl IntoView {

    if web_sys::window().unwrap().navigator().bluetooth().is_none(){
        return view! {
            <div class="container mx-auto">
                <div class="object-center text-center py-80 text-3xl">
                    "To use this site you'll need to use a browser that supports connecting to bluetooth devices, such as Chrome on a laptop or desktop computer"
                </div>
            </div>
        }
    };

    let (history, set_history, _) =
        use_local_storage::<BTreeMap<u32, ReadLogs>, JsonCodec>("history");

    let (state, set_state) = create_signal(ConnectionState::Disconnected);
    let (get_chart, set_chart) = create_signal(None);

    let get_characteristics_and_listeners = create_action(move |_| {

        async move {
            get_characteristics_and_listeners_from_service(NODE_UART_UUID, UART_RX_CHARACTERISTIC_UUID, UART_TX_CHARACTERISTIC_UUID, set_state, state, set_history, )
                .await
        }
    });

    create_resource(|| (), move |_| async move { 
        loop {
            request_log_updates(history, state, get_characteristics_and_listeners).await;
            TimeoutFuture::new(1_000).await;
        }
    });

    let _history_update = watch_throttled(move|| history.get(), 
    move |history, _, _| {
        chart_handler(history, set_chart, get_chart, state);
    }, 1000.0);

    request_idle_callback(move || {
        chart_handler(&history.get_untracked(), set_chart, get_chart, state);
    });

    setup_disconnect_handler(set_state);

    view! {
        <div class="container mx-auto w-[1200px]">
            <div class="flex flex-col justify-center mt-40">
                <div class="flex flex-row place-content-center">
                    <div class="basis-3/4">
                        <div id="chart" class="chart"></div>
                    </div>
                    <div class="flex flex-col place-content-between">
                        <div class="basis-3/4">
                            <components::LiveTempContainer state=state></components::LiveTempContainer>
                        </div>
                        <div class="justify-self-end">
                            <div class="flex flex-row">

                                <Button
                                    class="mx-10"
                                    variant=ButtonVariant::Primary
                                    disabled=create_memo(move |_| {
                                        !matches!(state.get(), ConnectionState::Disconnected)
                                    })

                                    loading=create_memo(move |_| {
                                        matches!(state.get(), ConnectionState::Connecting)
                                    })

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
                                        set_history.set(BTreeMap::new());
                                    }
                                >

                                    "Reset Data"
                                </Button>
                            </div>
                        </div>
                    </div>
                </div>

            </div>

        </div>
    }

}

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    leptos::mount_to_body(|| view! { <App/> })
}
