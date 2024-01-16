mod bluetooth;
mod chart;
mod history;

use std::{collections::BTreeMap, panic};

use bluetooth::{get_characteristics_and_listeners_from_service, show_connected, ConnectionState};
use history::LogItem;
use leptos::*;

use leptos_use::storage::{use_local_storage, JsonCodec};
use stylers::style;
use thaw::{Button, ButtonVariant, Layout, Spinner, SpinnerSize, Grid, GridItem};
use uuid::Uuid;

use crate::{bluetooth::CharacteristicArgs, chart::chart_handler, history::request_log_updates};

const NODE_UART_UUID: Uuid = uuid::uuid!("6E400001-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_RX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400002-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_TX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400003-B5A3-F393-E0A9-E50E24DCCA9E");

#[component]
fn App() -> impl IntoView {
    let stable = create_resource(|| (), |_| async move { show_connected().await });

    let (history, set_history, reset_history) =
        use_local_storage::<BTreeMap<u32, LogItem>, JsonCodec>("history");

    let (state, set_state) = create_signal(ConnectionState::Disconnected);
    let (get_chart, set_chart) = create_signal(None);

    let get_characteristics_and_listeners = create_action(move |args: &CharacteristicArgs| {
        let service = args.service.to_owned();
        let rx = args.rx_characteristic.to_owned();
        let tx = args.tx_characteristic.to_owned();
        async move {
            get_characteristics_and_listeners_from_service(service, rx, tx, set_state, set_history)
                .await
        }
    });

    let _async_result = move || {
        stable
            .get()
            .map(|value| format!("Server returned {value:?}"))
            // This loading state will only show before the first load
            .unwrap_or_else(|| "Loading...".into())
    };

    let leptos_use::UseIntervalReturn { counter, .. } = leptos_use::use_interval(1000);

    let _refresh_effect = create_effect(move |_| {
        counter.get();

        spawn_local(async move {
            request_log_updates(history, state, get_characteristics_and_listeners).await
        });
    });

    let _history_update = create_effect(move |_| {
        let history = history.get();

        chart_handler(history, set_chart, get_chart);
    });

    let styler_class = style! {
    .temperature {
        color:red;
        padding-bottom: 40px;
        margin-left: auto;
        margin-right: auto;
        font-size: 90px;
    }
    .grid {
        display: grid;
        place-content: center;
        border-width: 10px;
        height: 100vh;
    }
    };

    view! { class=styler_class,
        <Layout style="max-width: 800px; margin-left: auto; margin-right: auto;">
            <div id="chart" class="chart"></div>
            <Grid cols=2>
                <GridItem column=2>

                    {move || match state.get() {
                        ConnectionState::Disconnected => {
                            view! { <p>"Waiting for connection..."</p> }.into_view()
                        }
                        ConnectionState::Connecting => {
                            view! { <Spinner size=SpinnerSize::Medium/> }.into_view()
                        }
                        ConnectionState::Connected(s) => {
                            view! { class=styler_class,
                                <div id="temperature" class="temperature">
                                    {format!("{:.1}C", s.temperature)}
                                </div>
                            }
                                .into_view()
                        }
                    }}

                </GridItem>
                <GridItem>
                    <Button
                        variant=ButtonVariant::Primary
                        on:click=move |_| {
                            get_characteristics_and_listeners
                                .dispatch(CharacteristicArgs {
                                    service: NODE_UART_UUID,
                                    rx_characteristic: UART_RX_CHARACTERISTIC_UUID,
                                    tx_characteristic: UART_TX_CHARACTERISTIC_UUID,
                                });
                        }
                    >

                        "Connect"
                    </Button>
                </GridItem>
                <GridItem class="willcenter">
                    <Button
                        variant=ButtonVariant::Primary
                        style="place-self: center;"
                        on:click=move |_| {
                            reset_history();
                            set_history.set(BTreeMap::new());
                        }
                    >

                        "Reset"
                    </Button>
                </GridItem>
            </Grid>
        </Layout>
    }
}

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    leptos::mount_to_body(|| view! { <App/> })
}
