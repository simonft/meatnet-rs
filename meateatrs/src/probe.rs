use anyhow::{anyhow, Result};
use btleplug::{
    api::{Characteristic, Peripheral as _},
    platform::{Peripheral, PeripheralId},
};
use deku::{DekuContainerRead as _, DekuContainerWrite as _};
use futures::{Future, StreamExt as _};
use meatnet::{
    uart::{self, probe::response::Response},
    ProbeStatus,
};
use range_set_blaze::RangeSetBlaze;
use std::{collections::BTreeMap, pin::Pin, time::Duration};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::time::sleep;
use uuid::Uuid;

use crate::combustion_device::{CombustionDevice, SyncMessages};

const PROBE_STATUS_CHARACTERISTIC: Uuid = uuid::uuid!("00000101-CAAB-3792-3D44-97AE51C1407A");

pub struct Probe {
    peripheral_id: PeripheralId,
    bluetooth_handle: Option<Peripheral>,
    messages_rx: Receiver<SyncMessages>,
    messages_tx: Sender<SyncMessages>,
    rx_characteristic: Option<Characteristic>,
}

impl Probe {
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

impl CombustionDevice for Probe {
    fn set_bluetooth_handle(&mut self, thermometer: Peripheral) {
        self.bluetooth_handle = Some(thermometer);
    }
    fn set_rx_characteristic(&mut self, rx_characteristic: Characteristic) {
        self.rx_characteristic = Some(rx_characteristic);
    }
    fn get_device_type(&self) -> meatnet::ProductType {
        meatnet::ProductType::PredictiveProbe
    }

    fn get_peripheral_id(&self) -> &PeripheralId {
        &self.peripheral_id
    }

    fn process_notifications(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            let mut history = BTreeMap::new();

            let thermometer = match &self.bluetooth_handle {
                Some(bluetooth_handle) => bluetooth_handle,
                None => return Err(anyhow!("Bluetooth handle not set (call .setup() first)")),
            };

            let mut notification_stream = thermometer.notifications().await?;

            // Process while the BLE connection is not broken or stopped.
            // Unlike the node, frequently there are multiple messages received in one notification, at least on Linux.
            // Also the ProbeStatus request in request_probe_log_updates() seems to trigger a weird unparsable message.
            while let Some(data) = notification_stream.next().await {
                let mut offset = 0usize;
                let mut rest = data.value.as_slice();

                loop {
                    let result = Response::from_bytes((rest, offset));

                    match result {
                        Ok(((tmp_rest, tmp_offset), message)) => {
                            match message.message {
                                uart::probe::response::ResponseMessage::ReadLogs(m) => {
                                    history.insert(m.sequence_number, m.temperatures);
                                    self.messages_tx
                                        .send(SyncMessages::LogRecieved(m.sequence_number))?;
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
            Ok(())
        })
    }

    fn request_log_updates(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            let mut logs_received: RangeSetBlaze<u32> = range_set_blaze::RangeSetBlaze::new();

            let thermometer = match &self.bluetooth_handle {
                Some(bluetooth_handle) => bluetooth_handle,
                None => return Err(anyhow!("Bluetooth handle not set (call .setup() first)")),
            };

            let rx_characteristic = match &self.rx_characteristic {
                Some(rx_characteristic) => rx_characteristic,
                None => return Err(anyhow!("Rx characteristic not set (call .setup() first)")),
            };

            let characteristics = thermometer.characteristics();
            let probe_status_characteristic = characteristics
                .iter()
                .find(|c| c.uuid == PROBE_STATUS_CHARACTERISTIC)
                .expect("Unable to find probe status characteristic");

            let mut messages_rx = self.messages_rx.resubscribe();
            loop {
                sleep(Duration::from_millis(1000)).await;

                while let Ok(message) = messages_rx.try_recv() {
                    if let SyncMessages::LogRecieved(sequence_number) = message {
                        logs_received.insert(sequence_number);
                        if sequence_number % 10 == 0 {
                            println!("Sequence number: {}", sequence_number);
                        }
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
                            uart::probe::request::RequestType::ReadLogs(
                                uart::probe::request::ReadLogs {
                                    sequence_number_start: start,
                                    sequence_number_end: end,
                                },
                            ),
                        );

                        let data = read_logs
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
        })
    }
}
