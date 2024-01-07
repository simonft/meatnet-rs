pub mod uart;

use bitvec::prelude::*;
use deku::prelude::*;
use std::u8;
use uart::request::NetworkInformation;

#[cfg(test)]
use crate::uart::request::Hops;

#[cfg(test)]
use pretty_assertions::assert_eq;

#[derive(Debug, PartialEq, DekuRead)]
pub struct Temperature {
    raw_value: u16,
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(magic = b"\x09\xc7")]
pub struct ManufacturerSpecificData {
    pub product_type: ProductType,
    pub probe_serial_number: SerialNumber,
    #[deku(reader = "parse_raw_temperature_data(deku::rest)")]
    pub temperatures: [Temperature; 8],
    #[deku(bits = "3")]
    pub probe_id: u8,
    pub color: Color,
    pub mode: Mode,
    #[deku(bits = "2")]
    virtual_ambient_sensor: u8,
    #[deku(bits = "2")]
    virtual_surface_sensor: u8,
    #[deku(bits = "3")]
    virtual_core_sensor: u8,
    pub battery_status: BatteryStatus,
    #[deku(
        cond = "product_type == &ProductType::MeatNetRepeater",
        default = "None",
        pad_bytes_after = "match product_type {
            ProductType::MeatNetRepeater => 1,
            _ => 2,
        }"
    )]
    pub network_information: Option<NetworkInformation>,
}

impl ManufacturerSpecificData {
    pub fn get_core_temperature(&self) -> &Temperature {
        &self.temperatures[self.virtual_core_sensor as usize]
    }

    pub fn get_surface_temperature(&self) -> &Temperature {
        &self.temperatures[self.virtual_surface_sensor as usize + 3]
    }

    pub fn get_ambient_temperature(&self) -> &Temperature {
        &self.temperatures[self.virtual_ambient_sensor as usize + 4]
    }
}

impl Temperature {
    pub fn new(raw_value: u16) -> Self {
        Temperature { raw_value }
    }

    pub fn get_celsius(&self) -> f32 {
        (self.raw_value as f32 * 0.05) - 20.0
    }

    pub fn get_fahrenheit(&self) -> f32 {
        (self.get_celsius() * 9.0 / 5.0) + 32.0
    }
}

fn parse_raw_temperature_data(
    input: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, [Temperature; 8]), DekuError> {
    let (rest, bytes) = <[u8; 13]>::read(input, ())?;
    match bytes
        .into_bitarray::<Lsb0>()
        .chunks(13)
        .map(|chunk| (Temperature::new(chunk.load_le())))
        .collect::<Vec<Temperature>>()
        .try_into()
    {
        Ok(raw_temperatures) => Ok((rest, raw_temperatures)),
        Err(_) => Err(DekuError::Parse(
            "Unable to parse raw temperatures".to_string(),
        )),
    }
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct ProbeStatus {
    #[deku(endian = "little")]
    pub log_start: u32,
    #[deku(endian = "little")]
    pub log_end: u32,
    #[deku(reader = "parse_raw_temperature_data(deku::rest)")]
    temperatures: [Temperature; 8],
    #[deku(bits = "3")]
    pub probe_id: u8,
    pub color: Color,
    pub mode: Mode,
    #[deku(bits = "2")]
    virtual_ambient_sensor: u8,
    #[deku(bits = "2")]
    virtual_surface_sensor: u8,
    #[deku(bits = "3")]
    virtual_core_sensor: u8,
    #[deku(pad_bytes_after = "25")]
    pub battery_status: BatteryStatus,
}

impl ProbeStatus {
    pub fn get_core_temperature(&self) -> &Temperature {
        &self.temperatures[self.virtual_core_sensor as usize]
    }

    pub fn get_surface_temperature(&self) -> &Temperature {
        &self.temperatures[self.virtual_surface_sensor as usize + 3]
    }

    pub fn get_ambient_temperature(&self) -> &Temperature {
        &self.temperatures[self.virtual_ambient_sensor as usize + 4]
    }
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "2")]
pub enum Mode {
    Normal = 0,
    InstantRead,
    Reserved,
    Errored,
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "3")]
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
pub enum BatteryStatus {
    Ok = 0,
    LowBattery,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
pub enum ProductType {
    Unknown = 0,
    PredictiveProbe,
    MeatNetRepeater,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct SerialNumber {
    pub number: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct MacAddress {
    pub address: [u8; 6],
}

#[test]
fn test_parse_raw_temperature_data() {
    let data = [
        0x4a, 0x63, 0x69, 0x2c, 0x8d, 0xa5, 0x31, 0x35, 0xaa, 0x46, 0xd5, 0xc0, 0x1a,
    ];

    let (rest, raw_temperatures) = match parse_raw_temperature_data(BitSlice::from_slice(&data)) {
        Ok((rest, raw_temperatures)) => (rest, raw_temperatures),
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
    assert_eq!(rest.len(), 0);
}

#[test]
fn test_probe_status() {
    let data = [
        0x00, 0x00, 0x00, 0x00, 0x63, 0x00, 0x00, 0x00, 0x4a, 0x63, 0x69, 0x2c, 0x8d, 0xa5, 0x31,
        0x35, 0xaa, 0x46, 0xd5, 0xc0, 0x1a, 0x00, 0xc0, 0x00, 0x00, 0x00, 0xf0, 0xff, 0xbf, 0x34,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];

    let (_, probe_status) = ProbeStatus::from_bytes((&data, 0)).unwrap();
    assert_eq!(
        probe_status,
        ProbeStatus {
            log_start: 0,
            log_end: 99,
            temperatures: [
                Temperature::new(842),
                Temperature::new(843),
                Temperature::new(843),
                Temperature::new(843),
                Temperature::new(851),
                Temperature::new(853),
                Temperature::new(853),
                Temperature::new(856),
            ],
            probe_id: 0,
            color: Color::Yellow,
            mode: Mode::Normal,
            virtual_ambient_sensor: 3,
            virtual_surface_sensor: 0,
            virtual_core_sensor: 0,
            battery_status: BatteryStatus::Ok,
        }
    );
}

#[test]
fn test_manufacturer_specific_data() {
    let node_data = vec![
        0x09, 0xC7, 0x02, 0xed, 0x1d, 0x00, 0x10, 0x5c, 0x03, 0x6d, 0xb8, 0x0d, 0xb7, 0x11, 0x37,
        0xe2, 0xc6, 0xd9, 0xf8, 0x1a, 0x00, 0xc0, 0x00, 0x00,
    ];

    assert_eq!(
        ManufacturerSpecificData {
            probe_serial_number: SerialNumber { number: 0x10001ded },
            product_type: ProductType::MeatNetRepeater,
            temperatures: [
                Temperature::new(860),
                Temperature::new(872),
                Temperature::new(878),
                Temperature::new(878),
                Temperature::new(881),
                Temperature::new(881),
                Temperature::new(871),
                Temperature::new(863),
            ],
            probe_id: 0,
            color: Color::Yellow,
            mode: Mode::Normal,
            virtual_ambient_sensor: 3,
            virtual_surface_sensor: 0,
            virtual_core_sensor: 0,
            battery_status: BatteryStatus::Ok,
            network_information: Some(NetworkInformation {
                hop_count: Hops::One
            }),
        },
        ManufacturerSpecificData::from_bytes((node_data.as_slice(), 0))
            .unwrap()
            .1,
    );

    let probe_data = vec![
        0x09, 0xC7, 0x01, 0xed, 0x1d, 0x00, 0x10, 0xc7, 0x84, 0x97, 0xdc, 0x92, 0x51, 0x12, 0x47,
        0x84, 0xc8, 0x06, 0x71, 0x1f, 0x00, 0xc2, 0x00, 0x00,
    ];

    assert_eq!(
        ManufacturerSpecificData {
            probe_serial_number: SerialNumber { number: 0x10001ded },
            product_type: ProductType::PredictiveProbe,
            temperatures: [
                Temperature::new(1223),
                Temperature::new(1212),
                Temperature::new(1207),
                Temperature::new(1187),
                Temperature::new(1137),
                Temperature::new(1090),
                Temperature::new(1051),
                Temperature::new(1006),
            ],
            probe_id: 0,
            color: Color::Yellow,
            mode: Mode::Normal,
            virtual_ambient_sensor: 3,
            virtual_surface_sensor: 0,
            virtual_core_sensor: 1,
            battery_status: BatteryStatus::Ok,
            network_information: None,
        },
        ManufacturerSpecificData::from_bytes((probe_data.as_slice(), 0))
            .unwrap()
            .1,
    );
}
