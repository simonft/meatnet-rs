use std::collections::BTreeMap;

use js_sys::Array;
use uuid::Uuid;
use web_sys::{
    self, js_sys,
    wasm_bindgen::{self, closure::Closure, JsCast as _, JsValue},
    BluetoothDevice, BluetoothLeScanFilterInit, BluetoothRemoteGattCharacteristic,
    BluetoothRemoteGattService, Event, RequestDeviceOptions,
};

const NODE_UART_UUID: Uuid = uuid::uuid!("6E400001-B5A3-F393-E0A9-E50E24DCCA9E");

use meatnet::{
    temperature::Temperature,
    uart::node::{
        request::RequestMessage,
        response::{ReadLogs, ResponseMessage},
        try_request_or_response_from, MessageType,
    },
    Mode, SerialNumber,
};

use leptos::{ev, logging, prelude::*};

#[derive(Clone)]
pub struct CurrentState {
    pub core_temperature: Temperature,
    pub surface_temperature: Temperature,
    pub ambient_temperature: Temperature,
    pub serial_number: SerialNumber,
    pub log_start: u32,
    pub log_end: u32,
    pub mode: Mode,
}

#[derive(Clone)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected(CurrentState),
}

pub fn process_bluetooth_event(
    event: ev::CustomEvent,
    set_state: WriteSignal<ConnectionState>,
    get_state: ReadSignal<ConnectionState>,
    set_history: WriteSignal<BTreeMap<u32, ReadLogs>>,
) {
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

    match try_request_or_response_from(vec_data.as_slice()) {
        Ok(message) => match message {
            #[allow(clippy::single_match)]
            MessageType::Response(r) => match r.message {
                ResponseMessage::ReadLogs(m) => {
                    // We only support one probe right now.
                    if let ConnectionState::Connected(current_state) = get_state.get_untracked() {
                        if current_state.serial_number == m.probe_serial_number {
                            set_history.update(|history| {
                                history.insert(m.sequence_number, m);
                            });
                        }
                    }
                }
                _ => (),
            },
            MessageType::Request(r) => match r.message {
                RequestMessage::HeartbeatMessage(_) => {}
                RequestMessage::SyncThermometerList(_) => {}
                RequestMessage::ProbeStatusMessage(m) => {
                    // We only support one probe right now.
                    if m.status.mode == Mode::Normal && m.status.probe_id == 0 {
                        set_state.set(ConnectionState::Connected(CurrentState {
                            core_temperature: *m.status.get_core_temperature(),
                            surface_temperature: *m.status.get_surface_temperature(),
                            ambient_temperature: *m.status.get_ambient_temperature(),
                            log_start: m.status.log_start,
                            log_end: m.status.log_end,
                            serial_number: m.probe_serial_number,
                            mode: m.status.mode,
                        }));
                    }
                }
                _ => (),
            },
        },
        Err(e) => {
            logging::log!("{:02x?}", vec_data);
            logging::log!("error: {}", e);
        }
    }
}

pub fn setup_disconnect_handler(state: WriteSignal<ConnectionState>) {
    let bluetooth = web_sys::window().unwrap().navigator().bluetooth().unwrap();

    let disconnected_func = Closure::wrap(Box::new(move |_: Event| {
        state.set(ConnectionState::Disconnected);
        logging::log!("disconnected");
    }) as Box<dyn FnMut(Event)>);

    bluetooth.set_ongattserverdisconnected(Some(disconnected_func.as_ref().unchecked_ref()));

    disconnected_func.forget();
}

pub async fn get_service(
    uuid: &Uuid,
    set_state: WriteSignal<ConnectionState>,
) -> BluetoothRemoteGattService {
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

    let disconnected_func = Closure::wrap(Box::new(move |_: Event| {
        set_state.set(ConnectionState::Disconnected);
        logging::log!("disconnected");
    }) as Box<dyn FnMut(Event)>);

    device.set_ongattserverdisconnected(Some(disconnected_func.as_ref().unchecked_ref()));

    disconnected_func.forget();

    set_state.set(ConnectionState::Connecting);

    let gatt = device.gatt().unwrap();

    match wasm_bindgen_futures::JsFuture::from(gatt.connect()).await {
        Ok(_) => logging::log!("connected"),
        Err(e) => {
            logging::log!("error: {:?}", e);
            panic!("error: {:?}", e)
        }
    }
    BluetoothRemoteGattService::from(
        match wasm_bindgen_futures::JsFuture::from(
            gatt.get_primary_service_with_str(&uuid.to_string()),
        )
        .await
        {
            Ok(service) => service,
            Err(e) => {
                logging::log!("error: {:?}", e);
                panic!("error: {:?}", e)
            }
        },
    )
}

pub async fn get_characteristics_and_listeners_from_service(
    service: Uuid,
    rx_characteristic: Uuid,
    tx_characteristic: Uuid,
    set_state: WriteSignal<ConnectionState>,
    get_state: ReadSignal<ConnectionState>,
    set_history: WriteSignal<BTreeMap<u32, ReadLogs>>,
) -> BluetoothRemoteGattCharacteristic {
    let service = get_service(&service, set_state).await;

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
        process_bluetooth_event(ev, set_state, get_state, set_history)
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

    listener_func.forget();

    rx_characteristic
}
