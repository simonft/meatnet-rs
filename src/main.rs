use nom::character;
use uuid::Uuid;

use js_sys::Array;
use leptos_use;
use wasm_bindgen_futures::{self, wasm_bindgen::JsCast};
use web_sys::{
    self, js_sys, BluetoothDevice, BluetoothLeScanFilterInit, BluetoothRemoteGattCharacteristic,
    BluetoothRemoteGattService, RequestDeviceOptions,
};

pub mod types;
use types::uart;
use types::ProbeStatus;

const PROBE_STATUS_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("00000101-CAAB-3792-3D44-97AE51C1407A");
const PROBE_STATUS_SERVICE_UUID: Uuid = uuid::uuid!("00000100-CAAB-3792-3D44-97AE51C1407A");
const NODE_UART_UUID: Uuid = uuid::uuid!("6E400001-B5A3-F393-E0A9-E50E24DCCA9E");
const BLUETOOTH_BASE_UUID: u128 = 0x00000000_0000_1000_8000_00805f9b34fb;
const PROBE_STATUS_UART_SERVICE_UUID: Uuid =
    Uuid::from_u128(BLUETOOTH_BASE_UUID | ((0x181A as u128) << 96));

const UART_RX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400002-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_TX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400003-B5A3-F393-E0A9-E50E24DCCA9E");

use leptos::*;

fn process_bluetooth_event(event: ev::CustomEvent) {
    logging::log!("{:#?}", event);

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
    logging::log!("{:#?}", vec_data);

    let probe_status = ProbeStatus::try_from(vec_data.as_slice());
    logging::log!("{:#?}", probe_status);
}

async fn get_service(uuid: &Uuid) -> BluetoothRemoteGattService {
    let bluetooth = web_sys::window().unwrap().navigator().bluetooth().unwrap();

    let services = Array::new();
    services.push(&PROBE_STATUS_SERVICE_UUID.to_string().into());

    let mut filter: BluetoothLeScanFilterInit = BluetoothLeScanFilterInit::new();
    filter.services(&services.into());

    let filters = Array::new();
    filters.push(&filter.into());

    let mut device_options = RequestDeviceOptions::new();
    device_options.filters(&filters.into());

    let device = BluetoothDevice::from(
        wasm_bindgen_futures::JsFuture::from(bluetooth.request_device(&device_options))
            .await
            .unwrap(),
    );

    let gatt = device.gatt().unwrap();

    wasm_bindgen_futures::JsFuture::from(gatt.connect())
        .await
        .unwrap();

    logging::log!("here1");

    BluetoothRemoteGattService::from(
        wasm_bindgen_futures::JsFuture::from(gatt.get_primary_service_with_str(&uuid.to_string()))
            .await
            .unwrap(),
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
) -> CharacteristicsAndListenerResult {
    let service = get_service(&service).await;

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

    let listener_func = wasm_bindgen::closure::Closure::wrap(
        Box::new(process_bluetooth_event) as Box<dyn FnMut(_)>
    );

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

    let get_characteristics_and_listeners = create_action(|args: &CharacteristicArgs| {
        let service = args.service.to_owned();
        let rx = args.rx_characteristic.to_owned();
        let tx = args.tx_characteristic.to_owned();
        async move { get_characteristics_and_listeners_from_service(service, rx, tx).await }
    });

    let async_result = move || {
        stable
            .get()
            .map(|value| format!("Server returned {value:?}"))
            // This loading state will only show before the first load
            .unwrap_or_else(|| "Loading...".into())
    };

    let request = types::uart::Request {
        message: &types::uart::ReadSessionInformation {},
    };
    let mut request_bytes = request.to_bytes().unwrap();
    logging::log!("Request Bytes:");
    request_bytes
        .iter()
        .for_each(|b| logging::log!("{:02x}", b));

    let leptos_use::UseIntervalReturn { counter, .. } = leptos_use::use_interval(1000);

    let refresh_effect = create_effect(move |_| {
        if let Some(result) = get_characteristics_and_listeners.value().get() {
            logging::log!("{:#?}", "running");
            spawn_local(async move {
                let request = types::uart::Request {
                    message: &types::uart::ReadSessionInformation {},
                };
                let mut request_bytes = request.to_bytes().unwrap();
                logging::log!("Request Bytes:");
                request_bytes
                    .iter()
                    .for_each(|b| logging::log!("{:02x}", b));
                let return_value = wasm_bindgen_futures::JsFuture::from(
                    result
                        .rx_characteristic
                        .write_value_without_response_with_u8_array(request_bytes.as_mut_slice()),
                )
                .await
                .unwrap();
                logging::log!("{:#?}", return_value);
            });
        };
    });

    view! {
        <button
            on:click= move |_| {
                get_characteristics_and_listeners.dispatch(CharacteristicArgs{
                    service: NODE_UART_UUID,
                    rx_characteristic: UART_RX_CHARACTERISTIC_UUID,
                    tx_characteristic: UART_TX_CHARACTERISTIC_UUID
                });
            }
        >
            "Connect"
        </button>

        <p>
        <code>"async_value"</code>": "
        {async_result}
        <br/>
    </p>

    }
}

fn main() {
    leptos::mount_to_body(|| view! { <App/> })
}
