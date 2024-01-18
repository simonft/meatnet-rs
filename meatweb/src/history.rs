use deku::DekuContainerWrite as _;
use gloo::timers::future::TimeoutFuture;
use leptos::{Action, ReadSignal, Signal, SignalGetUntracked};
use meatnet::{uart::node::request, uart::node::response, EncapsulatableMessage as _, SerialNumber};
use range_set_blaze::RangeSetBlaze;
use std::collections::BTreeMap;

use crate::bluetooth::{CharacteristicArgs, CharacteristicsAndListenerResult, ConnectionState};


pub async fn request_log_updates(
    history: Signal<BTreeMap<u32, response::ReadLogs>>,
    connection_state: ReadSignal<ConnectionState>,
    characteristics: Action<CharacteristicArgs, CharacteristicsAndListenerResult>,
) {
    logging::log!("starting request_log_updates");

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

            let mut data = request::ReadLogs {
                probe_serial_number: SerialNumber { number: state.serial_number },
                sequence_number_start: start,
                sequence_number_end: end,
            }
            .encapsulate()
            .to_bytes()
            .expect("Could not create ReadLogs message");

            wasm_bindgen_futures::JsFuture::from(
                rx_characteristic.write_value_without_response_with_u8_array(data.as_mut_slice()),
            )
            .await
            .expect("Connected to bluetooth");

            TimeoutFuture::new(1_000).await;
        }
    }
}
