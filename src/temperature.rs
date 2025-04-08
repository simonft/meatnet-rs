use alloc::{borrow::Cow, format, string::ToString, vec::Vec};
use bitvec::prelude::*;
use deku::{
    ctx::BitSize,
    no_std_io::{Read, Seek},
    prelude::*,
    DekuReader,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use deku::no_std_io::Cursor;
#[cfg(test)]
use pretty_assertions::assert_eq;

pub trait IsTemperature {
    fn get_celsius(&self) -> f32;

    fn get_fahrenheit(&self) -> f32 {
        (self.get_celsius() * 9.0 / 5.0) + 32.0
    }
}

#[derive(Debug, PartialEq, DekuRead, Clone, Copy, Serialize, Deserialize)]
pub struct Temperature {
    raw_value: u16,
}

impl Temperature {
    pub fn new(raw_value: u16) -> Self {
        Temperature { raw_value }
    }

    pub fn get_raw_value(&self) -> u16 {
        self.raw_value
    }
}

impl IsTemperature for Temperature {
    fn get_celsius(&self) -> f32 {
        (self.raw_value as f32 * 0.05) - 20.0
    }

    fn get_fahrenheit(&self) -> f32 {
        (self.get_celsius() * 9.0 / 5.0) + 32.0
    }
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct CoreTemperature {
    #[deku(bits = "11", endian = "little")]
    raw_value: u16,
}

impl CoreTemperature {
    pub fn new(raw_value: u16) -> Self {
        Self { raw_value }
    }
}

impl IsTemperature for CoreTemperature {
    fn get_celsius(&self) -> f32 {
        (self.raw_value as f32 * 0.1) - 20.0
    }
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct PredictionSetPointTemperature {
    #[deku(bits = "10", endian = "little")]
    raw_value: u16,
}

impl PredictionSetPointTemperature {
    pub fn new(raw_value: u16) -> Self {
        Self { raw_value }
    }
}

impl IsTemperature for PredictionSetPointTemperature {
    fn get_celsius(&self) -> f32 {
        self.raw_value as f32 * 0.1
    }
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct HeatStartTemperature {
    #[deku(bits = "10", endian = "little")]
    raw_value: u16,
}

impl HeatStartTemperature {
    pub fn new(raw_value: u16) -> Self {
        Self { raw_value }
    }
}

impl IsTemperature for HeatStartTemperature {
    fn get_celsius(&self) -> f32 {
        self.raw_value as f32 * 0.1
    }
}

pub fn parse_raw_temperature_data<R: Read + Seek>(
    reader: &mut Reader<R>,
    bit_size: BitSize,
) -> Result<[Temperature; 8], DekuError> {
    let bytes = <[u8; 13]>::from_reader_with_ctx(reader, bit_size)?;

    match bytes
        .into_bitarray::<Lsb0>()
        .chunks(13)
        .map(|chunk| (Temperature::new(chunk.load_le())))
        .collect::<Vec<Temperature>>()
        .try_into()
    {
        Ok(raw_temperatures) => Ok(raw_temperatures),
        Err(e) => Err(DekuError::Parse(Cow::from(
            format!("Unable to parse raw temperatures: {:?}", e).to_string(),
        ))),
    }
}

#[test]
fn test_parse_raw_temperature_data() {
    let data: [u8; 13] = [
        0x4a, 0x63, 0x69, 0x2c, 0x8d, 0xa5, 0x31, 0x35, 0xaa, 0x46, 0xd5, 0xc0, 0x1a,
    ];
    let mut cursor = Cursor::new(data);

    let mut reader = Reader::new(&mut cursor);

    let raw_temperatures = match parse_raw_temperature_data(&mut reader, BitSize(8 * 13)) {
        Ok(raw_temperatures) => raw_temperatures,
        Err(e) => panic!("Error: {}", e),
    };
    assert_eq!(
        raw_temperatures,
        [
            Temperature::new(842),
            Temperature::new(843),
            Temperature::new(843),
            Temperature::new(843),
            Temperature::new(851),
            Temperature::new(853),
            Temperature::new(853),
            Temperature::new(856),
        ]
    );
}
