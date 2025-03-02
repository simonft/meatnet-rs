extern crate alloc;

use alloc::format;
#[cfg(test)]
use alloc::vec;
use alloc::{borrow::Cow, string::ToString};
use deku::{
    ctx::BitSize,
    no_std_io::{Seek, Write},
    prelude::*,
};

use crate::{parse_raw_temperature_data, Temperature};

#[derive(Debug, PartialEq, DekuRead)]
pub struct SetProbeId {}

#[derive(Debug, PartialEq, DekuRead)]
pub struct ReadSessionInformation {
    probe_session_id: u32,
    probe_sample_period: u16,
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct ReadLogs {
    pub sequence_number: u32,
    #[deku(reader = "parse_raw_temperature_data(deku::reader, BitSize(8*13))")]
    pub temperatures: [Temperature; 8],
    pub virtual_sensors_and_state: [u8; 7],
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(ctx = "response_type: u8", id = "response_type")]
pub enum ResponseMessage {
    #[deku(id = "0x01")]
    SetProbeId(SetProbeId),
    #[deku(id = "0x03")]
    ReadSessionInformation(ReadSessionInformation),
    #[deku(id = "0x04")]
    ReadLogs(ReadLogs),
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(magic = b"\xca\xfe")]
pub struct Response {
    crc: u16,
    pub response_type: u8,
    #[deku(bytes = "1")]
    success: bool,
    payload_length: u8,
    #[deku(ctx = "*response_type")]
    pub message: ResponseMessage,
}

impl DekuWriter for Response {
    fn to_writer<W: Write + Seek>(&self, _: &mut Writer<W>, _: ()) -> Result<(), DekuError> {
        Err(DekuError::Parse(Cow::from("Not implimented".to_string())))
    }
}

#[test]
fn test_parse_read_session_information_response() {
    let data = vec![202, 254, 188, 168, 3, 1, 6, 188, 254, 245, 34, 136, 19];
    let (_extra, _message) = Response::from_bytes((data.as_slice(), 0)).unwrap();
    //TODO: Finish
}

#[test]
fn test_parse_read_logs_response() {
    let data = vec![
        0xca, 0xfe, 0x26, 0xb9, 0x04, 0x01, 0x18, 0x09, 0x00, 0x00, 0x00, 0x6e, 0xe5, 0xad, 0x98,
        0x95, 0xa6, 0x82, 0x50, 0x88, 0x89, 0x24, 0x69, 0x23, 0x70, 0x00, 0x00, 0xfe, 0xff, 0xd7,
        0x0a,
    ];

    let (_, response) = Response::from_bytes((data.as_slice(), 0)).unwrap();

    assert_eq!(
        response,
        Response::from_bytes((data.as_slice(), 0)).unwrap().1
    );
}
