use std::{fmt, pin::Pin};

use anyhow::{anyhow, Result};
use btleplug::{
    api::{Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter},
    platform::{Adapter, Manager, Peripheral, PeripheralId},
};
use futures::{Future, StreamExt};
use meatnet::{ManufacturerSpecificData, ProductType};
use uuid::Uuid;

use crate::{node::Node, probe::Probe};

const COMBUSTION_BLUETOOTH_ID: u16 = 0x09C7;
const UART_RX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400002-B5A3-F393-E0A9-E50E24DCCA9E");
const UART_TX_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("6E400003-B5A3-F393-E0A9-E50E24DCCA9E");

#[derive(Clone, Debug)]
pub enum SyncMessages {
    LogRangeAvailble(u32, u32),
    LogRecieved(u32),
}

#[derive(Debug)]
pub struct FindDeviceError {
    details: String,
}

impl fmt::Display for FindDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for FindDeviceError {
    fn description(&self) -> &str {
        &self.details
    }
}

pub async fn find_combustion_device(
    central: &Adapter,
    product_type: Option<&ProductType>,
) -> Result<Box<dyn CombustionDevice>> {
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
            for (key, mut value) in manufacturer_data {
                if key == COMBUSTION_BLUETOOTH_ID {
                    println!("Found Combustion device: {:?}", id);
                    let mut magic: Vec<u8> = vec![0x09, 0xC7];
                    magic.append(&mut value);

                    match ManufacturerSpecificData::try_from(magic.as_slice()) {
                        Ok(data) => {
                            if product_type == Some(&data.product_type) || product_type.is_none() {
                                match &data.product_type {
                                    ProductType::MeatNetRepeater => {
                                        return Ok(Box::new(Node::new(id)?));
                                    }
                                    ProductType::PredictiveProbe => {
                                        return Ok(Box::new(Probe::new(id)?));
                                    }
                                    _ => {
                                        println!(
                                            "Found unknown device type: {:?}",
                                            data.product_type
                                        );
                                    }
                                }
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
    Err(anyhow!("Unable to find version"))
}

pub trait CombustionDevice: Send + Sync {
    fn get_device_type(&self) -> ProductType;
    fn process_notifications(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn request_log_updates(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn set_bluetooth_handle(&mut self, thermometer: Peripheral);
    fn set_rx_characteristic(&mut self, rx_characteristic: Characteristic);
    fn get_peripheral_id(&self) -> &PeripheralId;

    fn setup(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + '_>> {
        Box::pin(async move {
            let manager = Manager::new().await.unwrap();

            // get the first bluetooth adapter
            let central = manager
                .adapters()
                .await
                .expect("Unable to fetch adapter list.")
                .into_iter()
                .next()
                .expect("Unable to find adapters.");

            let handle = central.peripheral(self.get_peripheral_id()).await.unwrap();
            handle.connect().await?;
            handle.discover_services().await?;

            // Setup characteristics
            let characteristics = handle.characteristics();

            let tx_characteristic = match characteristics
                .iter()
                .find(|c| c.uuid == UART_TX_CHARACTERISTIC_UUID)
            {
                Some(c) => c,
                None => return Err(anyhow!("Unable to find tx characteristic")),
            };
            handle.subscribe(tx_characteristic).await?;

            self.set_bluetooth_handle(handle);

            self.set_rx_characteristic(
                match characteristics
                    .iter()
                    .find(|c| c.uuid == UART_RX_CHARACTERISTIC_UUID)
                {
                    Some(c) => c.clone(),
                    None => return Err(anyhow!("Unable to find rx characteristic")),
                },
            );
            Ok(())
        })
    }
}
