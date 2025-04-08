use alloc::format;
use deku::ctx::BitSize;
use deku::prelude::*;

use crate::{
    common_types::{BatteryStatus, Color, Mode},
    temperature::{parse_raw_temperature_data, Temperature},
};

#[derive(Debug, PartialEq, DekuRead)]
pub struct ProbeStatus {
    #[deku(endian = "little")]
    pub log_start: u32,
    #[deku(endian = "little")]
    pub log_end: u32,
    #[deku(reader = "parse_raw_temperature_data(deku::reader, BitSize(8*13))")]
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
