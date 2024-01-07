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
use deku::DekuContainerWrite;
use range_set_blaze::RangeSetBlaze;
use tokio::task;

use meatnet::{uart, ManufacturerSpecificData, ProductType, SerialNumber};

const UART_RX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400002-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_TX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400003-B5A3-F393-E0A9-E50E24DCCA9E");

enum SyncMessages {
    LogRangeAvailble(u32, u32),
    LogRecieved(u32),
}

async fn process_notifications(thermometer: Peripheral, tx: mpsc::Sender<SyncMessages>) {
    let mut history = HashMap::new();

    let mut notification_stream = thermometer
        .notifications()
        .await
        .expect("Unable to get notifications.");
    // Process while the BLE connection is not broken or stopped.
    while let Some(data) = notification_stream.next().await {
        match uart::NodeMessage::try_from(data.value.as_slice()) {
            Ok(message) => match message.message_type {
                uart::MessageType::Response(r) => match r.message {
                    uart::response::ResponseMessage::ReadLogs(m) => {
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
                uart::MessageType::Request(r) => match r.message {
                    uart::request::RequestType::HeartbeatMessage(_) => {}
                    uart::request::RequestType::SyncThermometerList(_) => {}
                    uart::request::RequestType::ProbeStatusMessage(m) => {
                        tx.send(SyncMessages::LogRangeAvailble(
                            m.status.log_start,
                            m.status.log_end,
                        ))
                        .unwrap();
                    }
                    _ => {
                        //println!("Request: {:#?}", r);
                    }
                },
            },
            Err(e) => println!("error: {}", e),
        }
    }
}

async fn request_log_updates(
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

            for mut range in missing_logs.ranges() {
                while !range.is_empty() {
                    let start = *range.start();
                    let end = range.nth(num_request_concurrent).unwrap_or(*range.end());
                    let read_logs = uart::request::RequestType::ReadLogs(uart::request::ReadLogs {
                        probe_serial_number: SerialNumber { number: 0x10001DED },
                        sequence_number_start: start,
                        sequence_number_end: end,
                    });

                    let nm = uart::NodeMessage::new(uart::MessageType::Request(
                        uart::request::Request::new(read_logs),
                    ));

                    match thermometer
                        .write(
                            &rx_characteristic,
                            &nm.to_bytes().expect("Could not create ReadLogs message"),
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
    product_type: Option<ProductType>,
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
                            match product_type {
                                Some(ref pt) => {
                                    if data.product_type == *pt {
                                        return Some(id);
                                    };
                                }
                                None => {
                                    return Some(id);
                                }
                            };
                        }
                        Err(e) => println!(
                            "Expected to be able to parse ManufacturerSpecificData but can't: {}",
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
            &find_combustion_device(&central, Some(ProductType::MeatNetRepeater))
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

    let listener = task::spawn(process_notifications(thermometer.clone(), tx));
    let _requestor = task::spawn(request_log_updates(
        thermometer.clone(),
        rx_characteristic,
        rx,
    ));

    listener.await?;

    Ok(())
}
