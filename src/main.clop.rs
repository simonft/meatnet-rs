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
use nom::Parser;
use nom::{
    bits, bytes,
    character::complete::{char, digit1},
    combinator::map,
    multi::fill,
    number::complete::{le_u16, le_u32},
    sequence::{pair, tuple},
    IResult,
};
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
    fn parse13bits(i: BitInput) -> IResult<BitInput, u16> {
        let mut raw_value = 0;
        let mut rest = i;
        let mut output: u16;
        println!();
        for _ in 0..13 {
            (rest, output) = bits::complete::take(1usize)(rest)?;
            println!("{}", output);
            raw_value = raw_value * 2 + output;
        }

        println!("{raw_value}");
        Ok((rest, raw_value))
    }

    let mut buf = [0; 8];

    let (rest, ()) = bits::bits(fill(parse13bits, &mut buf))(bytes)?;
    let computed = buf.map(|x| x as f32 * 0.05 - 20.0);
    Ok((rest, computed))
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
        data.value.iter().for_each(|j| print!("{j:08b} "));
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
