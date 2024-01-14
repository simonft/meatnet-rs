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

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(magic = b"\xca\xfe")]
pub struct ResponseHeader {
    crc: u16,
    #[deku(assert = "*response_type >> 7 == 1")]
    response_type: u8,
    #[deku(endian = "little")]
    pub request_id: u32,
    #[deku(endian = "little")]
    pub response_id: u32,
    pub success: bool,
    payload_length: u8,
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct Response {
    pub header: ResponseHeader,
    #[deku(ctx = "header.response_type & 0b01111111")]
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

#[test]
fn test_wont_parse_request() {
    let expected = vec![
        0xca, 0xfe, 0xe9, 0xb5, 0x03, 0x42, 0xcd, 0x50, 0xa8, 0x04, 0xed, 0x1d, 0x00, 0x10,
    ];

    assert_eq!(
        Err(DekuError::Assertion(
            "ResponseHeader.response_type field failed assertion: * response_type >> 7 == 1".into()
        )),
        Response::from_bytes((expected.as_slice(), 0))
    )
}
