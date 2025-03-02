extern crate alloc;

#[cfg(test)]
use alloc::vec;
use alloc::{format, vec::Vec};
use deku::prelude::*;

use crate::SerialNumber;

mod readlogs;
pub use readlogs::ReadLogs;

#[derive(Debug, PartialEq, DekuRead)]
struct SetProbeId {}

#[derive(Debug, PartialEq, DekuRead)]
pub struct ReadSessionInformation {
    pub probe_serial_number: SerialNumber,
    pub probe_session_id: u32,
    pub probe_sample_period: u16,
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

#[test]
fn test_wont_parse_request() {
    let data = vec![
        0xca, 0xfe, 0xe9, 0xb5, 0x03, 0x42, 0xcd, 0x50, 0xa8, 0x04, 0xed, 0x1d, 0x00, 0x10,
    ];

    assert_eq!(
        Err(DekuError::Assertion(
            "ResponseHeader.response_type field failed assertion: * response_type >> 7 == 1".into()
        )),
        Response::from_bytes((data.as_slice(), 0))
    )
}
