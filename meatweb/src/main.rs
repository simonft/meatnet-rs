
use std::option;

use js_sys::Array;
use stylers::style;
use thaw::{Button, ButtonVariant, Layout, Spinner, SpinnerSize};
use uuid::Uuid;
use wasm_bindgen_futures::{self, wasm_bindgen::JsCast};
use web_sys::{
    self, js_sys,
    wasm_bindgen::{JsValue},
    BluetoothDevice, BluetoothLeScanFilterInit,
    BluetoothRemoteGattCharacteristic, BluetoothRemoteGattService, RequestDeviceOptions,
};


const NODE_UART_UUID: Uuid = uuid::uuid!("6E400001-B5A3-F393-E0A9-E50E24DCCA9E");

const UART_RX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400002-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_TX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400003-B5A3-F393-E0A9-E50E24DCCA9E");

use meatnet::{
    temperature::IsTemperature,
    uart::node::{
        request::{self, RequestMessage},
        response::ResponseMessage,
        try_request_or_response_from, MessageType,
    },
    SerialNumber,
};

use leptos::*;

#[derive(Clone)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected(f32),
}

fn process_bluetooth_event(event: ev::CustomEvent, set_temperature: WriteSignal<ConnectionState>) {
    let data = event
        .target()
        .unwrap()
        .dyn_ref::<BluetoothRemoteGattCharacteristic>()
        .unwrap()
        .value()
        .unwrap();

    let mut vec_data = Vec::new();

    for i in 0..data.byte_length() {
        vec_data.push(data.get_uint8(i));
    }
    logging::log!("{:02x?}", vec_data);

    match try_request_or_response_from(vec_data.as_slice()) {
        Ok(message) => match message {
            MessageType::Response(r) => match r.message {
                ResponseMessage::ReadLogs(m) => {
                    logging::log!("{:#?}", m)
                }
                _ => logging::log!("{:#?}", r),
            },
            MessageType::Request(r) => match r.message {
                RequestMessage::HeartbeatMessage(_) => {}
                RequestMessage::SyncThermometerList(_) => {}
                RequestMessage::ProbeStatusMessage(m) => {
                    set_temperature(ConnectionState::Connected(m.status.get_core_temperature().get_celsius()));
                }
                _ => logging::log!("{:#?}", r),
            },
        },
        Err(e) => logging::log!("error: {}", e),
    }
}

async fn get_service(uuid: &Uuid, set_temperature: WriteSignal<ConnectionState>) -> BluetoothRemoteGattService {
    let _predictive_probe_id = 1;
    let _meatnet_repeater_id = 2;

    let bluetooth = web_sys::window().unwrap().navigator().bluetooth().unwrap();

    let manufacturer_hash = js_sys::Object::new();
    js_sys::Reflect::set(
        &manufacturer_hash,
        &JsValue::from("companyIdentifier"),
        &JsValue::from(0x09C7),
    )
    .expect("could not reflect");
    js_sys::Reflect::set(
        &manufacturer_hash,
        &JsValue::from("dataPrefix"),
        &JsValue::from(js_sys::Uint8Array::from([_meatnet_repeater_id].as_slice())),
    )
    .expect("Could not reflect");
    let manufacturer_data = web_sys::js_sys::Array::new();
    manufacturer_data.push(&JsValue::from(manufacturer_hash));

    let mut filter: BluetoothLeScanFilterInit = BluetoothLeScanFilterInit::new();
    filter.manufacturer_data(&js_sys::Object::from(manufacturer_data));

    let filters = Array::new();
    filters.push(&filter.into());

    let mut device_options = RequestDeviceOptions::new();
    device_options.filters(&filters.into());
    let optional_services = web_sys::js_sys::Array::new();
    optional_services.push(&JsValue::from(NODE_UART_UUID.to_string()));

    device_options.optional_services(&js_sys::Object::from(optional_services));

    let device = BluetoothDevice::from(
        wasm_bindgen_futures::JsFuture::from(bluetooth.request_device(&device_options))
            .await
            .unwrap(),
    );

    set_temperature(ConnectionState::Connecting);

    let gatt = device.gatt().unwrap();

    match wasm_bindgen_futures::JsFuture::from(gatt.connect())
        .await {
            Ok(_) => logging::log!("connected"),
            Err(e) => {
                logging::log!("error: {:?}", e);
                panic!("error: {:?}", e)
            },
        }
    BluetoothRemoteGattService::from(
        match wasm_bindgen_futures::JsFuture::from(gatt.get_primary_service_with_str(&uuid.to_string())).await {
            Ok(service) => service,
            Err(e) => {
                logging::log!("error: {:?}", e);
                panic!("error: {:?}", e)
            },
        }
    )
}

#[derive(Clone)]
struct CharacteristicsAndListenerResult {
    service: BluetoothRemoteGattService,
    rx_characteristic: BluetoothRemoteGattCharacteristic,
    tx_characteristic: BluetoothRemoteGattCharacteristic,
}

async fn get_characteristics_and_listeners_from_service(
    service: Uuid,
    rx_characteristic: Uuid,
    tx_characteristic: Uuid,
    set_temperature: WriteSignal<ConnectionState>
) -> CharacteristicsAndListenerResult {
    let service = get_service(&service, set_temperature).await;

    let rx_characteristic = BluetoothRemoteGattCharacteristic::from(
        wasm_bindgen_futures::JsFuture::from(
            service.get_characteristic_with_str(&rx_characteristic.to_string()),
        )
        .await
        .unwrap(),
    );

    let tx_characteristic = BluetoothRemoteGattCharacteristic::from(
        wasm_bindgen_futures::JsFuture::from(
            service.get_characteristic_with_str(&tx_characteristic.to_string()),
        )
        .await
        .unwrap(),
    );

    let listener_func = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev| {
        process_bluetooth_event(ev, set_temperature)
    }) as Box<dyn FnMut(_)>);

    tx_characteristic
        .add_event_listener_with_callback(
            "characteristicvaluechanged",
            listener_func.as_ref().unchecked_ref(),
        )
        .unwrap();

    wasm_bindgen_futures::JsFuture::from(tx_characteristic.start_notifications())
        .await
        .unwrap();

    logging::log!("{:#?}", "finished connecting");

    listener_func.forget();

    CharacteristicsAndListenerResult {
        service,
        rx_characteristic,
        tx_characteristic,
    }
}

async fn show_connected() -> Vec<String> {
    let bluetooth = web_sys::window().unwrap().navigator().bluetooth().unwrap();
    Array::from(
        &wasm_bindgen_futures::JsFuture::from(bluetooth.get_devices())
            .await
            .unwrap(),
    )
    .into_iter()
    .map(|device| {
        let device = BluetoothDevice::from(device);
        format!("{} - {}", device.name().unwrap(), device.id())
    })
    .collect()
}

async fn send_data(characteristic: BluetoothRemoteGattCharacteristic, data: Vec<u8>) {}

#[component]
fn App() -> impl IntoView {
    let stable = create_resource(|| (), |_| async move { show_connected().await });
    struct CharacteristicArgs {
        service: Uuid,
        tx_characteristic: Uuid,
        rx_characteristic: Uuid,
    }

    let (temperature, set_temperature) = create_signal(ConnectionState::Disconnected);

    let get_characteristics_and_listeners = create_action(move |args: &CharacteristicArgs| {
        let service = args.service.to_owned();
        let rx = args.rx_characteristic.to_owned();
        let tx = args.tx_characteristic.to_owned();
        async move {
            get_characteristics_and_listeners_from_service(service, rx, tx, set_temperature).await
        }
    });

    let async_result = move || {
        stable
            .get()
            .map(|value| format!("Server returned {value:?}"))
            // This loading state will only show before the first load
            .unwrap_or_else(|| "Loading...".into())
    };

    let leptos_use::UseIntervalReturn { counter, .. } = leptos_use::use_interval(1000);

    let refresh_effect = create_effect(move |_| {
        if let Some(result) = get_characteristics_and_listeners.value().get() {
            logging::log!("{:#?}", "running");
            spawn_local(async move {
                let read_logs = RequestMessage::ReadLogs(request::ReadLogs {
                    probe_serial_number: SerialNumber { number: 0x10001DED },
                    sequence_number_start: 0,
                    sequence_number_end: 2,
                });
                let mut request_bytes = read_logs
                    .to_bytes()
                    .expect("Could not convert message to bytes");
                logging::log!("Request Bytes:");
                logging::log!("{:02x?}", request_bytes);
                let return_value = wasm_bindgen_futures::JsFuture::from(
                    result
                        .rx_characteristic
                        .write_value_without_response_with_u8_array(request_bytes.as_mut_slice()),
                )
                .await
                .expect("Connected to bluetooth");
                logging::log!("{:#?}", return_value);
            });
        };
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
        <Layout style="max-width: 300px; margin-left: auto; margin-right: auto;">
            <div id="grid" class="grid">
                <div>
                    {move || match temperature.get() {
                        ConnectionState::Disconnected => {
                            view! { <p>"Waiting for connection..."</p> }.into_view()
                        }
                        ConnectionState::Connecting => {
                            view! { <Spinner size=SpinnerSize::Medium/> }.into_view()
                        }
                        ConnectionState::Connected(t) => {
                            view! { class=styler_class,
                                <div id="temperature" class="temperature">
                                    {format!("{:.1}C", t)}
                                </div>
                            }
                                .into_view()
                        }
                    }}

                </div>
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
            </div>

        </Layout>
    }
}

fn main() {
    leptos::mount_to_body(|| view! { <App/> })
}
