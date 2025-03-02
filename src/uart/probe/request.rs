use crc::{Crc, CRC_16_IBM_3740};
use deku::prelude::*;

use crate::EncapsulatableMessage;

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct SetProbeId {}
impl EncapsulatableMessage for SetProbeId {
    type Encapsulation = Request;
    fn encapsulate(self) -> Self::Encapsulation {
        Request::new(RequestType::SetProbeId(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct SetProbeColor {}
impl EncapsulatableMessage for SetProbeColor {
    type Encapsulation = Request;
    fn encapsulate(self) -> Self::Encapsulation {
        Request::new(RequestType::SetProbeColor(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct ReadSessionInformation {}
impl EncapsulatableMessage for ReadSessionInformation {
    type Encapsulation = Request;
    fn encapsulate(self) -> Self::Encapsulation {
        Request::new(RequestType::ReadSessionInformation(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct ReadLogs {
    pub sequence_number_start: u32,
    pub sequence_number_end: u32,
}
impl EncapsulatableMessage for ReadLogs {
    type Encapsulation = Request;
    fn encapsulate(self) -> Self::Encapsulation {
        Request::new(RequestType::ReadLogs(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(ctx = "request_type: u8", id = "request_type")]
pub enum RequestType {
    #[deku(id = "0x01")]
    SetProbeId(SetProbeId),
    #[deku(id = "0x02")]
    SetProbeColor(SetProbeColor),
    #[deku(id = "0x03")]
    ReadSessionInformation(ReadSessionInformation),
    #[deku(id = "0x04")]
    ReadLogs(ReadLogs),
}
impl RequestType {
    pub fn to_bytes(&self) -> Result<Vec<u8>, DekuError> {
        match self {
            RequestType::SetProbeId(r) => r.to_bytes(),
            RequestType::SetProbeColor(r) => r.to_bytes(),
            RequestType::ReadSessionInformation(r) => r.to_bytes(),
            RequestType::ReadLogs(r) => r.to_bytes(),
        }
    }

    // This could just return Request::new(self), but we're using it to make sure we've implemented
    // EncapsulatableMessage for all RequestMessage variants.
    pub fn encapsulate(self) -> Request {
        match self {
            RequestType::SetProbeId(r) => r.encapsulate(),
            RequestType::SetProbeColor(r) => r.encapsulate(),
            RequestType::ReadSessionInformation(r) => r.encapsulate(),
            RequestType::ReadLogs(r) => r.encapsulate(),
        }
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(magic = b"\xca\xfe")]
pub struct Request {
    crc: u16,
    pub request_type: u8,
    payload_length: u8,
    #[deku(ctx = "*request_type")]
    pub message: RequestType,
}

impl Request {
    pub fn new(message: RequestType) -> Self {
        let binding = Crc::<u16>::new(&CRC_16_IBM_3740);
        let mut digest = binding.digest();

        let request_response_type = message
            .deku_id()
            .expect("New message doesn't have Deku id.");
        let message_bytes = message.to_bytes().unwrap();

        // CRC of message type, request ID, payload length, and payload bytes.
        // TODO: when implementing response messages, this will need to be updated:
        // CRC of message type, request ID, response ID, success, payload length, and payload bytes
        digest.update(&[request_response_type]);
        digest.update(&[message_bytes.len() as u8]);
        digest.update(&message_bytes);

        Self {
            crc: digest.finalize(),
            request_type: message.deku_id().unwrap(),
            payload_length: message.to_bytes().unwrap().len() as u8,
            message,
        }
    }
}

#[test]
fn test_read_logs_request() {
    assert_eq!(
        Request::new(RequestType::ReadLogs(ReadLogs {
            sequence_number_start: 8,
            sequence_number_end: 10
        }))
        .to_bytes()
        .unwrap(),
        vec![0xca, 0xfe, 0x82, 0x13, 0x04, 0x08, 0x08, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00],
    )
}
