use deku::prelude::*;

use crate::{parse_raw_temperature_data, SerialNumber, Temperature};

#[derive(Debug, PartialEq, DekuRead)]
struct SetProbeId {}

#[derive(Debug, PartialEq, DekuRead)]
pub struct ReadSessionInformation {
    probe_serial_number: SerialNumber,
    probe_session_id: u32,
    probe_sample_period: u16,
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct ReadLogs {
    pub probe_serial_number: SerialNumber,
    pub sequence_number: u32,
    #[deku(reader = "parse_raw_temperature_data(deku::rest)")]
    pub temperatures: [Temperature; 8],
    pub virtual_sensors_and_state: [u8; 7],
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(ctx = "response_type: u8", id = "response_type")]
pub enum ResponseMessage {
    #[deku(id = "0x01")]
    SetProbeId,
    #[deku(id = "0x03")]
    ReadSessionInformation(ReadSessionInformation),
    #[deku(id = "0x04")]
    ReadLogs(ReadLogs),
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct Response {
    #[deku(bits = "7")]
    pub response_type: u8,
    #[deku(endian = "little")]
    request_id: u32,
    #[deku(endian = "little")]
    response_id: u32,
    #[deku(bytes = "1")]
    success: bool,
    payload_length: u8,
    #[deku(ctx = "*response_type")]
    pub message: ResponseMessage,
}

impl DekuWrite for Response {
    fn write(
        &self,
        _: &mut deku::bitvec::BitVec<u8, deku::bitvec::Msb0>,
        _: (),
    ) -> Result<(), DekuError> {
        Err(DekuError::Unexpected("Not implimented".to_string()))
    }
}
