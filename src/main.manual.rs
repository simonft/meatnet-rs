// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use bitvec::prelude::*;
use btleplug::api::Characteristic;
use btleplug::api::{
    bleuuid::uuid_from_u32, Central, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use rand::{thread_rng, Rng};
use std::error::Error;
use std::time::Duration;
use uuid::Uuid;

const PROBE_STATUS_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("00000101-CAAB-3792-3D44-97AE51C1407A");
use tokio::time;

#[derive(Debug)]
struct ProbeStatus {
    log_range: LogRange,
    raw_temperature_data: RawTemperatureData,
    mode_and_id: ModeAndId,
    // battery_status_and_virtual_sensors: BatteryStatusAndVirtualSensors,
    // prediction_status: PredictionStatus,
    // food_safe_data: FoodSafeData,
    // food_safe_status: FoodSafeStatus,
}

impl ProbeStatus {
    fn from_bytes(bytes: [u8; 48]) -> Self {
        Self {
            log_range: LogRange::from_bytes(&bytes[0..8].try_into().unwrap()),
            raw_temperature_data: RawTemperatureData::from_bytes(&bytes[8..21].try_into().unwrap()),
            mode_and_id: ModeAndId::from_byte(bytes[21]),
        }
    }
}

#[derive(Debug)]
struct LogRange {
    min_sequence_number: u32,
    max_sequence_number: u32,
}

impl LogRange {
    fn from_bytes(bytes: &[u8; 8]) -> Self {
        Self {
            min_sequence_number: u32::from_le_bytes(bytes[..4].try_into().unwrap()),
            max_sequence_number: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        }
    }
}

#[derive(Debug)]
enum Mode {
    Normal,
    InstantRead,
    Reserved,
    Error,
}

#[derive(Debug)]
struct ModeAndId {
    mode: Mode,
}

impl ModeAndId {
    fn from_byte(byte: u8) -> Self {
        dbg!(byte);
        Self {
            mode: match byte % 4 {
                0 => Mode::Normal,
                1 => Mode::InstantRead,
                2 => Mode::Reserved,
                3 => Mode::Error,
                u => panic!("Unknown value {u}!"),
            },
        }
    }
}

#[derive(Debug)]
struct RawTemperatureData {
    thermistors: [f32; 8],
}

impl RawTemperatureData {
    fn from_bytes(bytes: &[u8; 13]) -> Self {
        Self {
            thermistors: bytes
                .iter()
                .flat_map(|b| b.into_bitarray::<Lsb0>())
                .collect::<BitVec>()
                .chunks(13)
                .map(|chunk| chunk.load_le::<u16>())
                .map(|raw| (raw as f32 * 0.05) - 20.0)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
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

    let probe_status = ProbeStatus::from_bytes(
        thermometer
            .read(probe_status_characteristic)
            .await?
            .try_into()
            .unwrap(),
    );
    println!("{:#?}", probe_status);

    Ok(())
}
//00000100-caab-3792-3d44-97ae51c1407a
