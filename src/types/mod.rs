use bitvec::prelude::*;
use deku::ctx::Endian;
use deku::prelude::*;
use std::u8;

fn parse_raw_temperature_data(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, [f32; 8]), DekuError> {
    let (rest, bytes) = <[u8; 13]>::read(rest, ())?;
    match bytes
        .into_bitarray::<Lsb0>()
        .chunks(13)
        .map(|chunk| (chunk.load_le::<u16>() as f32 * 0.05) - 20.0)
        .collect::<Vec<f32>>()
        .try_into()
    {
        Ok(raw_temperatures) => Ok((rest, raw_temperatures)),
        Err(_) => Err(DekuError::Parse(
            "Unable to parse raw temperatures".to_string(),
        )),
    }
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(endian = "little")]
pub struct ProbeStatus {
    pub log_start: u32,
    pub log_end: u32,
    #[deku(reader = "parse_raw_temperature_data(deku::rest)")]
    raw_temperatures: [f32; 8],
    pub mode: Mode,
    pub color: Color,
    #[deku(bits = "3")]
    pub probe_id: u8,
    pub battery_status: BatteryStatus,
    #[deku(bits = "3")]
    virtual_core_sensor: u8,
    #[deku(bits = "2")]
    virtual_surface_sensor: u8,
    #[deku(bits = "2", pad_bytes_after = "25")]
    virtual_ambient_sensor: u8,
}

impl ProbeStatus {
    pub fn get_core_temperature(&self) -> f32 {
        self.raw_temperatures[self.virtual_core_sensor as usize]
    }

    pub fn get_surface_temperature(&self) -> f32 {
        self.raw_temperatures[self.virtual_surface_sensor as usize + 3]
    }

    pub fn get_ambient_temperature(&self) -> f32 {
        self.raw_temperatures[self.virtual_ambient_sensor as usize + 4]
    }
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "2")]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub enum Mode {
    Normal = 0,
    InstantRead,
    Reserved,
    Errored,
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "3")]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub enum Color {
    Yellow = 0,
    Grey,
    Reserved2,
    Reserved3,
    Reserved4,
    Reserved5,
    Reserved6,
    Reserved7,
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "1")]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub enum BatteryStatus {
    Ok = 0,
    LowBattery,
}
