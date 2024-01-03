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
use nom::{
    bits, bytes,
    character::complete::{char, digit1},
    combinator::map,
    multi::fill,
    number::complete::{le_u16, le_u32},
    sequence::{pair, tuple},
    IResult,
};
use nom_derive::Nom;
use rand::{thread_rng, Rng};
use std::error::Error;
use std::time::Duration;
use std::u8;
use uuid::Uuid;

const PROBE_STATUS_CHARACTERISTIC_UUID: Uuid = uuid::uuid!("00000101-CAAB-3792-3D44-97AE51C1407A");
use tokio::time;

type BitInput<'a> = (&'a [u8], usize);

#[derive(Debug)]
struct ProbeStatus {
    log_range: LogRange,
    raw_temperatures: [f32; 8],
}

fn parse_raw_temperature_data(bytes: &[u8]) -> IResult<&[u8], [f32; 8]> {
    let (rest, output) = bytes::complete::take(13usize)(bytes)?;

    let result = output
        .iter()
        .flat_map(|b| b.into_bitarray::<Lsb0>())
        .collect::<BitVec>()
        .chunks(13)
        .map(|chunk| chunk.load_le::<u16>())
        .map(|raw| (raw as f32 * 0.05) - 20.0)
        .collect::<Vec<_>>()
        .try_into();

    match result {
        Ok(raw_temperatures) => Ok((rest, raw_temperatures)),
        Err(_) => Err(nom::Err::Error(nom::error::Error {
            input: bytes,
            code: nom::error::ErrorKind::Fail,
        })),
    }
}

impl ProbeStatus {
    fn parse(bytes: &[u8]) -> IResult<&[u8], Self> {
        map(
            tuple((LogRange::parse, parse_raw_temperature_data)),
            |(log_range, raw_temperatures)| ProbeStatus {
                log_range,
                raw_temperatures,
            },
        )(bytes)
    }
}

#[derive(Debug)]
struct LogRange {
    min: u32,
    max: u32,
}
impl LogRange {
    fn parse(bytes: &[u8]) -> IResult<&[u8], Self> {
        map(pair(le_u32, le_u32), |(min, max)| LogRange { min, max })(bytes)
    }
}

#[derive(Debug)]
struct RawTemperatureData {
    thermistors: [f32; 8],
}

#[derive(Debug, Nom)]
enum Mode {
    Normal,
    InstantRead,
    Reserved,
    Error,
}

#[derive(Debug, Nom)]
enum Color {
    Yellow,
    Grey,
    Reserved2,
    Reserved3,
    Reserved4,
    Reserved5,
    Reserved6,
    Reserved7,
}

#[derive(Nom)]
#[nom(LittleEndian)]
struct ModeAndId {
    mode: Mode,
    color: Color,
    probe_id: u8,
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
        data.value
            .as_slice()
            .iter()
            .for_each(|j| print!("{j:08b} "));
        let probe_status = ProbeStatus::parse(data.value.as_slice());
        println!("{:#?}", probe_status);
        // if let Mode::Normal = probe_status.mode_and_id().mode() {
        //     println!("{:#?}", probe_status)
        // }

        println!("{}", i);
        i += 1;
    }

    Ok(())
}
