use anyhow::{anyhow, Result};
use btleplug::{
    api::{Characteristic, Peripheral as _},
    platform::{Peripheral, PeripheralId},
};
use deku::DekuContainerWrite as _;
use futures::{Future, StreamExt as _};
use meatnet::{
    uart::{
        self,
        node::{
            request::{self, RequestMessage},
            response::ResponseMessage,
            MessageType,
        },
    },
    EncapsulatableMessage as _, SerialNumber,
};
use range_set_blaze::RangeSetBlaze;
use std::{
    collections::{BTreeMap, HashMap},
    pin::Pin,
    time::Duration,
};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::time::sleep;

use crate::combustion_device::{CombustionDevice, SyncMessages};

pub struct Node {
    peripheral_id: PeripheralId,
    bluetooth_handle: Option<Peripheral>,
    messages_rx: Receiver<SyncMessages>,
    messages_tx: Sender<SyncMessages>,
    rx_characteristic: Option<Characteristic>,
}

impl Node {
    pub fn new(peripheral_id: PeripheralId) -> Result<Self> {
        let (messages_tx, messages_rx) = channel::<SyncMessages>(100);

        Ok(Self {
            peripheral_id,
            bluetooth_handle: None,
            messages_rx,
            messages_tx,
            rx_characteristic: None,
        })
    }
}

impl CombustionDevice for Node {
    fn set_bluetooth_handle(&mut self, thermometer: Peripheral) {
        self.bluetooth_handle = Some(thermometer);
    }
    fn set_rx_characteristic(&mut self, rx_characteristic: Characteristic) {
        self.rx_characteristic = Some(rx_characteristic);
    }

    fn get_device_type(&self) -> meatnet::ProductType {
        meatnet::ProductType::MeatNetRepeater
    }

    fn get_peripheral_id(&self) -> &PeripheralId {
        &self.peripheral_id
    }

    fn process_notifications(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            let mut history = HashMap::new();

            let thermometer = match &self.bluetooth_handle {
                Some(bluetooth_handle) => bluetooth_handle,
                None => return Err(anyhow!("Bluetooth handle not set (call .setup() first)")),
            };

            let mut notification_stream = thermometer
                .notifications()
                .await
                .expect("Unable to get notifications.");
            // Process while the BLE connection is not broken or stopped.
            while let Some(data) = notification_stream.next().await {
                match uart::node::try_request_or_response_from(data.value.as_slice()) {
                    Ok(message) => match message {
                        MessageType::Response(r) => match r.message {
                            ResponseMessage::ReadLogs(m) => {
                                history
                                    .entry(m.probe_serial_number.number)
                                    .or_insert(BTreeMap::new())
                                    .entry(m.sequence_number)
                                    .or_insert(m.temperatures);
                                self.messages_tx
                                    .send(SyncMessages::LogRecieved(m.sequence_number))?;
                                if m.sequence_number % 10 == 0 {
                                    println!("Sequence number: {}", m.sequence_number);
                                }
                            }
                            _ => println!("Response: {:#?}", r),
                        },
                        uart::node::MessageType::Request(r) => match r.message {
                            RequestMessage::HeartbeatMessage(_) => {}
                            RequestMessage::SyncThermometerList(_) => {}
                            RequestMessage::ProbeStatusMessage(m) => {
                                self.messages_tx.send(SyncMessages::LogRangeAvailble(
                                    m.status.log_start,
                                    m.status.log_end,
                                ))?;
                            }
                            _ => {}
                        },
                    },
                    Err(e) => println!("error: {}", e),
                }
            }
            Ok(())
        })
    }

    fn request_log_updates(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            let mut logs_received: RangeSetBlaze<u32> = range_set_blaze::RangeSetBlaze::new();
            let mut latest_range_avalible = None;

            let thermometer = match &self.bluetooth_handle {
                Some(bluetooth_handle) => bluetooth_handle,
                None => return Err(anyhow!("Bluetooth handle not set (call .setup() first)")),
            };

            let rx_characteristic = match &self.rx_characteristic {
                Some(rx_characteristic) => rx_characteristic,
                None => return Err(anyhow!("Rx characteristic not set (call .setup() first)")),
            };

            let mut messages_rx = self.messages_rx.resubscribe();
            loop {
                sleep(Duration::from_millis(1000)).await;

                while let Ok(message) = messages_rx.try_recv() {
                    match message {
                        SyncMessages::LogRangeAvailble(start, end) => {
                            latest_range_avalible = Some((start, end));
                        }
                        SyncMessages::LogRecieved(sequence_number) => {
                            logs_received.insert(sequence_number);
                            if sequence_number % 10 == 0 {
                                println!("Sequence number: {}", sequence_number);
                            }
                        }
                    }
                }

                if let Some((start, end)) = latest_range_avalible {
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

                            let data = request::ReadLogs {
                                probe_serial_number: SerialNumber { number: 0x10001DED },
                                sequence_number_start: start,
                                sequence_number_end: end,
                            }
                            .encapsulate()
                            .to_bytes()
                            .expect("Could not create ReadLogs message");

                            match thermometer
                                .write(
                                    rx_characteristic,
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
        })
    }
}
