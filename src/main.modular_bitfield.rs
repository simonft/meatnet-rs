// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use bitvec::prelude::*;
use btleplug::api::Characteristic;
use btleplug::api::{
    bleuuid::uuid_from_u32, Central, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::StreamExt;
use modular_bitfield::{bitfield, specifiers, BitfieldSpecifier, Specifier};
use rand::{thread_rng, Rng};
use std::error::Error;
use std::time::Duration;
use uuid::Uuid;

const PROBE_STATUS_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("00000101-CAAB-3792-3D44-97AE51C1407A");
use tokio::time;

#[derive(Debug, BitfieldSpecifier)]
enum BatteryStatus {
    OK,
    LowBattery,
}

#[bitfield(bits = 384)]
#[derive(Debug)]
struct ProbeStatus {
    log_range: LogRange,
    raw_temperature_data: RawTemperatureData,
    mode_and_id: ModeAndId,
    battery_status: BatteryStatus,
    virtual_sensors: VirtualSensors,
    prediction_status: specifiers::B56,
    food_safe_data: specifiers::B80,
    food_safe_status: specifiers::B32,
    future: specifiers::B32,
}

#[bitfield(bits = 64)]
#[derive(Debug, BitfieldSpecifier)]
struct LogRange {
    min_sequence_number: u32,
    max_sequence_number: u32,
}

#[derive(Debug, BitfieldSpecifier)]
#[bits = 2]
enum Mode {
    Normal,
    InstantRead,
    Reserved,
    Error,
}

#[derive(Debug, BitfieldSpecifier)]
enum Color {
    Yellow,
    Gray,
    TDB2,
    TBD3,
    TBD4,
    TBD5,
    TBD6,
    TBD7,
}

#[derive(Debug, BitfieldSpecifier)]
enum ProbeIdentifier {
    ID1,
    ID2,
    ID3,
    ID4,
    ID5,
    ID6,
    ID7,
    ID8,
}

#[bitfield(bits = 8)]
#[derive(Debug, BitfieldSpecifier)]
struct ModeAndId {
    mode: Mode,
    color: Color,
    probe_identifier: ProbeIdentifier,
}

#[bitfield(bits = 104)]
#[derive(Debug, BitfieldSpecifier)]
struct RawTemperatureData {
    thermistor1: specifiers::B13,
    thermistor2: specifiers::B13,
    thermistor3: specifiers::B13,
    thermistor4: specifiers::B13,
    thermistor5: specifiers::B13,
    thermistor6: specifiers::B13,
    thermistor7: specifiers::B13,
    thermistor8: specifiers::B13,
}

impl RawTemperatureData {
    fn get_celsius_from_value(value: u16) -> f32 {
        (0.05 * value as f32) - 20.0
    }
}

#[derive(Debug, BitfieldSpecifier)]
#[bits = 3]
enum VirtualCoreSensor {
    SensorT1,
    SensorT2,
    SensorT3,
    SensorT4,
    SensorT5,
    SensorT6,
    SensorT7,
}

#[derive(Debug, BitfieldSpecifier)]
enum VirtualSurfaceSensor {
    SensorT4,
    SensorT5,
    SensorT6,
    SensorT7,
}

#[derive(Debug, BitfieldSpecifier)]
enum VirtualAmbientSensor {
    SensorT5,
    SensorT6,
    SensorT7,
    SensorT8,
}

#[bitfield(filled = false)]
#[derive(Debug, BitfieldSpecifier)]
struct VirtualSensors {
    core: VirtualCoreSensor,
    surface_sensor: VirtualSurfaceSensor,
    ambient_sensor: VirtualAmbientSensor,
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

    // start scanning for devices
    central.start_scan(ScanFilter::default()).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    time::sleep(Duration::from_secs(2)).await;
    let maybe_thermometer = central
        .peripherals()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.address().to_string() == *"C2:71:04:91:14:D0");

    let thermometer = maybe_thermometer.unwrap();

    thermometer.connect().await?;
    thermometer.discover_services().await?;

    let characteristics = thermometer.characteristics();

    let probe_status_characteristic = characteristics
        .iter()
        .find(|c| c.uuid == PROBE_STATUS_CHARACTERISTIC_UUID)
        .expect("Unable to find probe status characteristic");
    thermometer.subscribe(probe_status_characteristic).await?;

    let mut i = 0;

    let mut notification_stream = thermometer.notifications().await?;
    // Process while the BLE connection is not broken or stopped.
    while let Some(data) = notification_stream.next().await {
        let probe_status = ProbeStatus::from_bytes(data.value.try_into().unwrap());
        if let Mode::Normal = probe_status.mode_and_id().mode() {
            println!("{:#?}", probe_status)
        }

        println!("{}", i);
        i += 1;
    }

    Ok(())
}
