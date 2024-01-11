use futures::stream::StreamExt;
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use btleplug::api::{
    Central as _, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter,
};
use btleplug::platform::{Adapter, Manager, Peripheral, PeripheralId};
use deku::{DekuContainerRead, DekuContainerWrite};
use range_set_blaze::RangeSetBlaze;
use tokio::task;

use meatnet::{uart, ManufacturerSpecificData, ProbeStatus, ProductType, SerialNumber};

const UART_RX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400002-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_TX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400003-B5A3-F393-E0A9-E50E24DCCA9E");
const PROBE_STATUS_CHARACTERISTIC: Uuid = uuid::uuid!("00000101-CAAB-3792-3D44-97AE51C1407A");

enum SyncMessages {
    LogRangeAvailble(u32, u32),
    LogRecieved(u32),
}

async fn process_node_notifications(thermometer: Peripheral, tx: mpsc::Sender<SyncMessages>) {
    let mut history = HashMap::new();

    let mut notification_stream = thermometer
        .notifications()
        .await
        .expect("Unable to get notifications.");
    // Process while the BLE connection is not broken or stopped.
    while let Some(data) = notification_stream.next().await {
        match uart::node::Message::try_from(data.value.as_slice()) {
            Ok(message) => match message.message_type {
                uart::node::MessageType::Response(r) => match r.message {
                    uart::node::response::ResponseMessage::ReadLogs(m) => {
                        history
                            .entry(m.probe_serial_number.number)
                            .or_insert(HashMap::new())
                            .entry(m.sequence_number)
                            .or_insert(m.temperatures);
                        tx.send(SyncMessages::LogRecieved(m.sequence_number))
                            .unwrap();
                        if m.sequence_number % 10 == 0 {
                            println!("Sequence number: {}", m.sequence_number);
                        }
                    }
                    _ => println!("Response: {:#?}", r),
                },
                uart::node::MessageType::Request(r) => match r.message {
                    uart::node::request::RequestType::HeartbeatMessage(_) => {}
                    uart::node::request::RequestType::SyncThermometerList(_) => {}
                    uart::node::request::RequestType::ProbeStatusMessage(m) => {
                        tx.send(SyncMessages::LogRangeAvailble(
                            m.status.log_start,
                            m.status.log_end,
                        ))
                        .unwrap();
                    }
                    _ => {}
                },
            },
            Err(e) => println!("error: {}", e),
        }
    }
}

async fn process_probe_notifications(
    thermometer: Peripheral,
    tx: mpsc::Sender<SyncMessages>,
    serial_number: SerialNumber,
) {
    let mut history = HashMap::new();

    let mut notification_stream = thermometer
        .notifications()
        .await
        .expect("Unable to get notifications.");

    // Process while the BLE connection is not broken or stopped.
    // Unlike the node, frequently there are multiple messages received in one notification, at least on Linux.
    // Also the ProbeStatus request in request_probe_log_updates() seems to trigger a weird unparseable message.
    while let Some(data) = notification_stream.next().await {
        let mut offset = 0usize;
        let mut rest = data.value.as_slice();

        loop {
            let result = uart::probe::response::Response::from_bytes((rest, offset));

            match result {
                Ok(((tmp_rest, tmp_offset), message)) => {
                    match message.message {
                        uart::probe::response::ResponseMessage::ReadLogs(m) => {
                            history
                                .entry(serial_number.number)
                                .or_insert(HashMap::new())
                                .entry(m.sequence_number)
                                .or_insert(m.temperatures);
                            tx.send(SyncMessages::LogRecieved(m.sequence_number))
                                .unwrap();
                            if m.sequence_number % 10 == 0 {
                                println!("Sequence number: {}", m.sequence_number);
                            }
                        }
                        _ => println!("Response: {:#?}", message),
                    };
                    (rest, offset) = (tmp_rest, tmp_offset);
                    if rest.is_empty() {
                        break;
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
}

async fn request_probe_log_updates(
    thermometer: Peripheral,
    rx_characteristic: Characteristic,
    rx: mpsc::Receiver<SyncMessages>,
) {
    let mut logs_received: RangeSetBlaze<u32> = range_set_blaze::RangeSetBlaze::new();

    let characteristics = thermometer.characteristics();
    let probe_status_characteristic = characteristics
        .iter()
        .find(|c| c.uuid == PROBE_STATUS_CHARACTERISTIC)
        .expect("Unable to find probe status characteristic");

    loop {
        sleep(Duration::from_millis(1000)).await;

        while let Ok(message) = rx.try_recv() {
            if let SyncMessages::LogRecieved(sequence_number) = message {
                logs_received.insert(sequence_number);
                println!("Log recieved: {}", sequence_number);
            }
        }

        let probe_status = ProbeStatus::from_bytes((
            thermometer
                .read(probe_status_characteristic)
                .await
                .expect("Could not read from probe status characteristic")
                .as_slice(),
            0usize,
        ))
        .unwrap()
        .1;

        let start = probe_status.log_start;
        let end = probe_status.log_end;

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

                let read_logs = uart::probe::request::Request::new(
                    uart::probe::request::RequestType::ReadLogs(uart::probe::request::ReadLogs {
                        sequence_number_start: start,
                        sequence_number_end: end,
                    }),
                );

                let data = read_logs
                    .to_bytes()
                    .expect("Could not create ReadLogs message");

                match thermometer
                    .write(
                        &rx_characteristic,
                        &data,
                        btleplug::api::WriteType::WithResponse,
                    )
                    .await
                {
                    Ok(_) => println!("Requested messages for {} - {}", start, end),
                    Err(e) => println!("error: {}", e),
                }
                sleep(Duration::from_millis(1000)).await;
            }
        }
    }
}

async fn request_node_log_updates(
    thermometer: Peripheral,
    rx_characteristic: Characteristic,
    rx: mpsc::Receiver<SyncMessages>,
) {
    let mut logs_received: RangeSetBlaze<u32> = range_set_blaze::RangeSetBlaze::new();
    let mut latest_range_avalible = None;

    loop {
        sleep(Duration::from_millis(1000)).await;

        while let Ok(message) = rx.try_recv() {
            match message {
                SyncMessages::LogRangeAvailble(start, end) => {
                    latest_range_avalible = Some((start, end));
                }
                SyncMessages::LogRecieved(sequence_number) => {
                    logs_received.insert(sequence_number);
                    println!("Log recieved: {}", sequence_number);
                }
            }
        }

        if let Some((start, end)) = latest_range_avalible {
            let missing_logs = RangeSetBlaze::from_iter([start..=end]) - &logs_received;

            println!("Missing logs: {:?}", missing_logs);

            let num_request_concurrent = 10;

            // Loop through the missing logs and request 10 logs at a time
            for mut range in missing_logs.ranges() {
                while !range.is_empty() {
                    let start = *range.start();
                    let end = range
                        .nth(num_request_concurrent - 1)
                        .unwrap_or(*range.end());

                    let read_logs =
                        uart::node::request::RequestType::ReadLogs(uart::node::request::ReadLogs {
                            probe_serial_number: SerialNumber { number: 0x10001DED },
                            sequence_number_start: start,
                            sequence_number_end: end,
                        });

                    let data = uart::node::Message::new(uart::node::MessageType::Request(
                        uart::node::request::Request::new(read_logs),
                    ))
                    .to_bytes()
                    .expect("Could not create ReadLogs message");

                    match thermometer
                        .write(
                            &rx_characteristic,
                            &data,
                            btleplug::api::WriteType::WithResponse,
                        )
                        .await
                    {
                        Ok(_) => println!("Requested messages for {} - {}", start, end),
                        Err(e) => println!("error: {}", e),
                    }
                    sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    }
}

async fn find_combustion_device(
    central: &Adapter,
    product_type: Option<&ProductType>,
) -> Option<PeripheralId> {
    // Each adapter has an event stream, we fetch via events(),
    // simplifying the type, this will return what is essentially a
    // Future<Result<Stream<Item=CentralEvent>>>.
    let mut events = central.events().await.expect("Could not get events");

    // start scanning for devices
    central
        .start_scan(ScanFilter::default())
        .await
        .expect("can't start scan");

    // Keep scanning until we find a the Combustion device type we want
    while let Some(event) = events.next().await {
        if let CentralEvent::ManufacturerDataAdvertisement {
            id,
            manufacturer_data,
        } = event
        {
            for (key, value) in &manufacturer_data {
                if *key == 0x09C7 {
                    println!("Found Combustion device: {:?}", id);
                    let mut magic: Vec<u8> = vec![0x09, 0xC7];
                    magic.append(&mut value.clone());

                    match ManufacturerSpecificData::try_from(magic.as_slice()) {
                        Ok(data) => {
                            // If there's a product type filter make sure this is the right one.
                            if let Some(pt) = product_type {
                                if data.product_type == *pt {
                                    return Some(id);
                                };
                            } else {
                                return Some(id);
                            };
                        }
                        Err(e) => println!(
                            "Expected to be able to parse manufacturer specific data but can't: {}",
                            e
                        ),
                    }
                }
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let device_type = ProductType::MeatNetRepeater;

    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let central = manager
        .adapters()
        .await
        .expect("Unable to fetch adapter list.")
        .into_iter()
        .next()
        .expect("Unable to find adapters.");

    let thermometer = central
        .peripheral(
            &find_combustion_device(&central, Some(&device_type))
                .await
                .unwrap(),
        )
        .await
        .unwrap();

    thermometer.connect().await?;
    thermometer.discover_services().await?;

    let characteristics = thermometer.characteristics();

    let tx_characteristic = characteristics
        .iter()
        .find(|c| c.uuid == UART_TX_CHARACTERISTIC_UUID)
        .expect("Unable to find tx characteristic");
    thermometer.subscribe(tx_characteristic).await?;

    let rx_characteristic = characteristics
        .iter()
        .find(|c| c.uuid == UART_RX_CHARACTERISTIC_UUID)
        .expect("Unable to find rx characteristic")
        .clone();

    let (tx, rx) = mpsc::channel::<SyncMessages>();

    let (listener, _requestor) = match device_type {
        ProductType::MeatNetRepeater => (
            task::spawn(process_node_notifications(thermometer.clone(), tx)),
            task::spawn(request_node_log_updates(thermometer, rx_characteristic, rx)),
        ),
        ProductType::PredictiveProbe => (
            task::spawn(process_probe_notifications(
                thermometer.clone(),
                tx,
                SerialNumber { number: 0x10001DED },
            )),
            task::spawn(request_probe_log_updates(
                thermometer,
                rx_characteristic,
                rx,
            )),
        ),
        ProductType::Unknown => panic!("Unknown device type"),
    };

    listener.await?;

    Ok(())
}
