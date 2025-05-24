use alloc::format;
use deku::prelude::*;

use crate::{
    common_types::{BatteryStatus, Color, Mode, NetworkInformation, ProductType, SerialNumber},
    temperature::Temperature,
};

#[cfg(test)]
use crate::common_types::Hops;
#[cfg(test)]
use alloc::vec;
#[cfg(test)]
use pretty_assertions::assert_eq;

#[derive(Debug, PartialEq, DekuRead)]
#[deku(magic = b"\xc7\x09")]
pub struct ManufacturerSpecificData {
    pub product_type: ProductType,
    pub probe_serial_number: SerialNumber,
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

#[test]
fn test_manufacturer_specific_data() {
    let node_data = vec![
        0xc7, 0x09, 0x02, 0xed, 0x1d, 0x00, 0x10, 0x5c, 0x03, 0x6d, 0xb8, 0x0d, 0xb7, 0x11, 0x37,
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
        0xc7, 0x09, 0x01, 0xed, 0x1d, 0x00, 0x10, 0xc7, 0x84, 0x97, 0xdc, 0x92, 0x51, 0x12, 0x47,
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
