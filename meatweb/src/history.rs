use deku::DekuContainerWrite as _;
use gloo::timers::future::TimeoutFuture;
use itertools::{Itertools as _, MinMaxResult::MinMax};
use leptos::{logging, Action, ReadSignal, Signal, SignalGet, SignalGetUntracked};
use meatnet::{uart::node::request::ReadLogs, EncapsulatableMessage as _, SerialNumber};
use range_set_blaze::RangeSetBlaze;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::bluetooth::{CharacteristicArgs, CharacteristicsAndListenerResult, ConnectionState};

#[derive(Serialize, Deserialize, Clone, PartialEq, Default, Debug, Eq)]
pub struct Temperature {
    raw_value: u16,
}

impl Temperature {
    pub fn new(raw_value: u16) -> Self {
        Temperature { raw_value }
    }

    pub fn get_raw_value(&self) -> u16 {
        self.raw_value
    }

    pub fn get_celsius(&self) -> f32 {
        (self.raw_value as f32 * 0.05) - 20.0
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default, Debug, Eq)]
pub struct LogItem {
    pub sequence_number: u32,
    pub temperature: Temperature,
}

// if let Some(result) = get_characteristics_and_listeners.value().get() {
//     logging::log!("{:#?}", "running");
//     spawn_local(async move {
//         let read_logs = ReadLogs {
//             probe_serial_number: SerialNumber { number: 0x10001DED },
//             sequence_number_start: 0,
//             sequence_number_end: 2,
//         }.encapsulate();
//         let mut request_bytes = read_logs
//             .to_bytes()
//             .expect("Could not convert message to bytes");
//         logging::log!("Request Bytes:");
//         logging::log!("{:02x?}", request_bytes);
//         let return_value = wasm_bindgen_futures::JsFuture::from(
//             result
//                 .rx_characteristic
//                 .write_value_without_response_with_u8_array(request_bytes.as_mut_slice()),
//         )
//         .await
//         .expect("Connected to bluetooth");
//         logging::log!("{:#?}", return_value);
//     });
// };

pub async fn request_log_updates(
    history: Signal<BTreeMap<u32, LogItem>>,
    connection_state: ReadSignal<ConnectionState>,
    characteristics: Action<CharacteristicArgs, CharacteristicsAndListenerResult>,
) {
    let logs_received: RangeSetBlaze<u32> =
        range_set_blaze::RangeSetBlaze::from_iter(history.get_untracked().keys());

    let rx_characteristic = match characteristics.value().get_untracked() {
        Some(result) => result.rx_characteristic,
        None => return,
    };

    let state = match connection_state.get_untracked() {
        ConnectionState::Connected(state) => state,
        _ => return,
    };

    let start = state.log_start;
    let end = state.log_end;
    let missing_logs = RangeSetBlaze::from_iter([start..=end]) - &logs_received;

    match missing_logs.len() {
        0 => println!("No missing logs"),
        _ => println!("Missing logs: {:?}", missing_logs),
    }
    let num_request_concurrent = 10;

    // Loop through the missing logs and request 10 logs at a time
    for mut range in missing_logs.ranges() {
        while !range.is_empty() {
            let start = *range.start();
            let end = range
                .nth(num_request_concurrent - 1)
                .unwrap_or(*range.end());

            let mut data = ReadLogs {
                probe_serial_number: SerialNumber { number: 0x10001DED },
                sequence_number_start: start,
                sequence_number_end: end,
            }
            .encapsulate()
            .to_bytes()
            .expect("Could not create ReadLogs message");

            let return_value = wasm_bindgen_futures::JsFuture::from(
                rx_characteristic.write_value_without_response_with_u8_array(data.as_mut_slice()),
            )
            .await
            .expect("Connected to bluetooth");

            logging::log!("{:#?}", return_value);
            TimeoutFuture::new(1_000).await;
        }
    }
}
