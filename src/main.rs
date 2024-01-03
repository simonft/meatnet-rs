use std::{thread, time};
use uuid::Uuid;

use js_sys::Array;
use wasm_bindgen_futures::{self, wasm_bindgen::JsCast};
use web_sys::{
    self, js_sys, BluetoothDevice, BluetoothLeScanFilterInit, BluetoothRemoteGattCharacteristic,
    BluetoothRemoteGattService, RequestDeviceOptions,
};

mod types;
use types::ProbeStatus;

const PROBE_STATUS_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("00000101-CAAB-3792-3D44-97AE51C1407A");

use leptos::*;

fn process_bluetooth_event(event: ev::CustomEvent) {
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

async fn get_bluetooth_device() {
    let bluetooth = web_sys::window().unwrap().navigator().bluetooth().unwrap();

    let services = Array::new();
    services.push(&"00000100-caab-3792-3d44-97ae51c1407a".to_string().into());

    let mut filter: BluetoothLeScanFilterInit = BluetoothLeScanFilterInit::new();
    filter.services(&services.into());

    let filters = Array::new();
    filters.push(&filter.into());

    let mut device_options = RequestDeviceOptions::new();
    device_options.filters(&filters.into());
    logging::log!("here");

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

    let service = BluetoothRemoteGattService::from(
        wasm_bindgen_futures::JsFuture::from(
            //gatt.get_primary_service_with_str("00000100-caab-3792-3d44-97ae51c1407a"),
            gatt.get_primary_service_with_str("6e400001-b5a3-f393-e0a9-e50e24dcca9e"),
        )
        .await
        .unwrap(),
    );

    logging::log!("here2");

    let characteristic = BluetoothRemoteGattCharacteristic::from(
        wasm_bindgen_futures::JsFuture::from(
            service.get_characteristic_with_str("00000101-caab-3792-3d44-97ae51c1407a"),
        )
        .await
        .unwrap(),
    );
    logging::log!("here3");

    let listener = wasm_bindgen::closure::Closure::wrap(
        Box::new(process_bluetooth_event) as Box<dyn FnMut(_)>
    );

    characteristic
        .add_event_listener_with_callback(
            "characteristicvaluechanged",
            listener.as_ref().unchecked_ref(),
        )
        .unwrap();

    listener.forget();

    wasm_bindgen_futures::JsFuture::from(characteristic.start_notifications())
        .await
        .unwrap();

    logging::log!("here4");
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

#[component]
fn App() -> impl IntoView {
    let stable = create_resource(|| (), |_| async move { show_connected().await });

    let async_result = move || {
        stable
            .get()
            .map(|value| format!("Server returned {value:?}"))
            // This loading state will only show before the first load
            .unwrap_or_else(|| "Loading...".into())
    };

    view! {
        <button
            on:click= |_| {
                spawn_local(get_bluetooth_device());
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
