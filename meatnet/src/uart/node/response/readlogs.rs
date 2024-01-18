use deku::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{parse_raw_temperature_data, temperature::Temperature, SerialNumber};

#[cfg(test)]
use crate::uart::node::response::{Response, ResponseHeader, ResponseMessage};

#[derive(Debug, PartialEq, DekuRead, Clone, Serialize, Deserialize)]
pub struct ReadLogs {
    pub probe_serial_number: SerialNumber,
    pub sequence_number: u32,
    #[deku(reader = "parse_raw_temperature_data(deku::rest)")]
    pub temperatures: [Temperature; 8],
    #[deku(bits = "2", pad_bits_before = "1")]
    pub virtual_ambient_sensor: u8,
    #[deku(bits = "2")]
    pub virtual_surface_sensor: u8,
    #[deku(bits = "3")]
    pub virtual_core_sensor: u8,
    pub virtual_sensors_and_state: [u8; 6],
}

impl ReadLogs {
    pub fn get_virtual_surface_temperature(&self) -> &Temperature {
        &self.temperatures[self.virtual_surface_sensor as usize + 3]
    }
    pub fn get_virtual_core_temperature(&self) -> &Temperature {
        &self.temperatures[self.virtual_core_sensor as usize]
    }
    pub fn get_vitrual_ambient_temperature(&self) -> &Temperature {
        &self.temperatures[self.virtual_ambient_sensor as usize + 4]
    }
}

#[test]
fn test_parse_read_logs_response() {
    let data = vec![
        0xca, 0xfe, 0xc2, 0x8, 0x84, 0xb3, 0x69, 0x4c, 0xa, 0x42, 0x4f, 0x95, 0x44, 0x1, 0x1c,
        0xed, 0x1d, 0x0, 0x10, 0x2, 0x0, 0x0, 0x0, 0x26, 0x24, 0x80, 0x5c, 0x90, 0x13, 0xc2, 0x3d,
        0x56, 0xc7, 0xe4, 0x98, 0x1c, 0xe0, 0x0, 0x0, 0xfe, 0xff, 0xd7, 0x7,
    ];
    let read_logs = ReadLogs {
        probe_serial_number: SerialNumber { number: 268443117 },
        sequence_number: 2,
        temperatures: [
            Temperature::new(1062),
            Temperature::new(1025),
            Temperature::new(1047),
            Temperature::new(1063),
            Temperature::new(988),
            Temperature::new(939),
            Temperature::new(915),
            Temperature::new(915),
        ],
        virtual_ambient_sensor: 3,
        virtual_surface_sensor: 0,
        virtual_core_sensor: 0,
        virtual_sensors_and_state: [0, 0, 254, 255, 215, 7],
    };

    let expected = Response {
        header: ResponseHeader {
            crc: 2242,
            response_type: 132,
            request_id: 172779955,
            response_id: 1150635842,
            success: true,
            payload_length: 28,
        },
        message: ResponseMessage::ReadLogs(read_logs),
    };
    assert_eq!(expected, Response::try_from(data.as_slice()).unwrap());
}

// Use the below (reversing the order of the fields after temperatures) when Deku supports lsb0.

// #[derive(Debug, PartialEq, DekuRead)]
// pub struct ReadLogs {
//     pub probe_serial_number: SerialNumber,
//     pub sequence_number: u32,
//     #[deku(reader = "parse_raw_temperature_data(deku::rest)")]
//     pub temperatures: [Temperature; 8],
//     // This part is reversed in the docs. Redo ordering when Deku supports lsb0.
//     #[deku(pad_bits_before = "3")]
//     pub estimated_core_temperature: CoreTemperature,
//     #[deku(
//         bits = "17",
//         map = "|input: u64| -> Result<_, DekuError> {Ok(Duration::from_secs(input))}",
//         endian = "little"
//     )]
//     pub prediction: Duration,
//     pub prediction_set_point_temperature: PredictionSetPointTemperature,
//     pub prediction_type: PredictionType,
//     pub prediction_mode: PredictionMode,
//     pub prediction_state: PredictionState,
//     #[deku(bits = "2")]
//     virtual_ambient_sensor: u8,
//     #[deku(bits = "2")]
//     virtual_surface_sensor: u8,
//     #[deku(bits = "3")]
//     virtual_core_sensor: u8,
// }

// impl ReadLogs {
//     #[allow(clippy::too_many_arguments)]
//     pub fn new(
//         probe_serial_number: SerialNumber,
//         sequence_number: u32,
//         temperatures: [Temperature; 8],
//         estimated_core_temperature: CoreTemperature,
//         prediction: Duration,
//         prediction_set_point_temperature: PredictionSetPointTemperature,
//         prediction_type: PredictionType,
//         prediction_mode: PredictionMode,
//         prediction_state: PredictionState,
//         virtual_ambient_sensor: u8,
//         virtual_surface_sensor: u8,
//         virtual_core_sensor: u8,
//     ) -> Self {
//         Self {
//             probe_serial_number,
//             sequence_number,
//             temperatures,
//             estimated_core_temperature,
//             prediction,
//             prediction_set_point_temperature,
//             prediction_type,
//             prediction_mode,
//             prediction_state,
//             virtual_ambient_sensor,
//             virtual_surface_sensor,
//             virtual_core_sensor,
//         }
//     }
//     pub fn get_core_temperature(&self) -> &Temperature {
//         &self.temperatures[self.virtual_core_sensor as usize]
//     }

//     pub fn get_surface_temperature(&self) -> &Temperature {
//         &self.temperatures[self.virtual_surface_sensor as usize + 3]
//     }

//     pub fn get_ambient_temperature(&self) -> &Temperature {
//         &self.temperatures[self.virtual_ambient_sensor as usize + 4]
//     }
// }

// #[test]
// fn test_parse_read_logs_response() {
//     let data = vec![
//         0xca, 0xfe, 0xc2, 0x8, 0x84, 0xb3, 0x69, 0x4c, 0xa, 0x42, 0x4f, 0x95, 0x44, 0x1, 0x1c,
//         0xed, 0x1d, 0x0, 0x10, 0x2, 0x0, 0x0, 0x0, 0x26, 0x24, 0x80, 0x5c, 0x90, 0x13, 0xc2, 0x3d,
//         0x56, 0xc7, 0xe4, 0x98, 0x1c, 0xe0, 0x0, 0x0, 0xfe, 0xff, 0xd7, 0x7,
//     ];
//     println!("{:02x?}", data);
//     let read_logs = ReadLogs::new(
//         SerialNumber { number: 268443117 },
//         2,
//         [
//             Temperature::new(1062),
//             Temperature::new(1025),
//             Temperature::new(1047),
//             Temperature::new(1063),
//             Temperature::new(988),
//             Temperature::new(939),
//             Temperature::new(915),
//             Temperature::new(915),
//         ],
//         CoreTemperature::new(1062),
//         Duration::from_secs(0),
//         PredictionSetPointTemperature::new(0),
//         PredictionType::None,
//         PredictionMode::None,
//         PredictionState::ProbeInserted,
//         0,
//         0,
//         0,
//     );

//     let expected = Response {
//         header: ResponseHeader {
//             crc: 2242,
//             response_type: 132,
//             request_id: 172779955,
//             response_id: 1150635842,
//             success: true,
//             payload_length: 28,
//         },
//         message: ResponseMessage::ReadLogs(read_logs),
//     };
//     assert_eq!(expected, Response::try_from(data.as_slice()).unwrap());
// }
