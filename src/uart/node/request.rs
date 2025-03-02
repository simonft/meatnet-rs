use crc::{Crc, CRC_16_IBM_3740};
use deku::prelude::*;

use crate::{MacAddress, NetworkInformation, ProbeStatus, ProductType, SerialNumber};

use crate::EncapsulatableMessage;

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct SetProbeId {}

impl EncapsulatableMessage for SetProbeId {
    type Encapsulation = Request;
    fn encapsulate(self) -> Request {
        Request::new(RequestMessage::SetProbeId(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct SetProbeColor {}

impl EncapsulatableMessage for SetProbeColor {
    type Encapsulation = Request;
    fn encapsulate(self) -> Request {
        Request::new(RequestMessage::SetProbeColor(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct ReadSessionInformation {
    pub serial_number: SerialNumber,
}

impl EncapsulatableMessage for ReadSessionInformation {
    type Encapsulation = Request;
    fn encapsulate(self) -> Request {
        Request::new(RequestMessage::ReadSessionInformation(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct ReadLogs {
    pub probe_serial_number: SerialNumber,
    pub sequence_number_start: u32,
    pub sequence_number_end: u32,
}

impl EncapsulatableMessage for ReadLogs {
    type Encapsulation = Request;
    fn encapsulate(self) -> Request {
        Request::new(RequestMessage::ReadLogs(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(type = "u8")]
pub enum Direction {
    Outbound = 0,
    Inbound,
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct Attributes {
    #[deku(bits = "1", pad_bits_before = "7")]
    pub connection_detail_record_is_populated: bool,
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct ProbeStatusMessage {
    pub probe_serial_number: SerialNumber,
    pub status: ProbeStatus,
    pub network_information: NetworkInformation,
}

impl DekuWrite for ProbeStatusMessage {
    fn write(
        &self,
        _: &mut deku::bitvec::BitVec<u8, deku::bitvec::Msb0>,
        _: (),
    ) -> Result<(), DekuError> {
        Err(DekuError::Unexpected("Not implimented".to_string()))
    }
}

impl EncapsulatableMessage for ProbeStatusMessage {
    type Encapsulation = Request;
    fn encapsulate(self) -> Request {
        Request::new(RequestMessage::ProbeStatusMessage(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct ConnectionDetailRecord {
    pub serial_number: [u8; 10],
    pub product_type: ProductType,
    pub attributes: Attributes,
    pub rssi: u8,
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct HeartbeatMessage {
    pub node_serial_number: [u8; 10],
    pub mac_address: MacAddress,
    pub product_type: ProductType,
    pub hop_count: u8,
    pub is_inbound: Direction,
    pub connection_details: [ConnectionDetailRecord; 4],
}

impl EncapsulatableMessage for HeartbeatMessage {
    type Encapsulation = Request;
    fn encapsulate(self) -> Request {
        Request::new(RequestMessage::HeartbeatMessage(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct SyncThermometer {
    #[deku(bytes = "1")]
    present: bool,
    serial_number: SerialNumber,
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct SyncThermometerList {
    mac_address: MacAddress,
    sync_thermometers: [SyncThermometer; 4],
}

impl EncapsulatableMessage for SyncThermometerList {
    type Encapsulation = Request;
    fn encapsulate(self) -> Request {
        Request::new(RequestMessage::SyncThermometerList(self))
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(ctx = "request_type: u8", id = "request_type")]
pub enum RequestMessage {
    #[deku(id = "0x01")]
    SetProbeId(SetProbeId),
    #[deku(id = "0x02")]
    SetProbeColor(SetProbeColor),
    #[deku(id = "0x03")]
    ReadSessionInformation(ReadSessionInformation),
    #[deku(id = "0x04")]
    ReadLogs(ReadLogs),
    #[deku(id = "0x45")]
    ProbeStatusMessage(ProbeStatusMessage),
    #[deku(id = "0x49")]
    HeartbeatMessage(HeartbeatMessage),
    #[deku(id = "0x4b")]
    SyncThermometerList(SyncThermometerList),
}
impl RequestMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, DekuError> {
        match self {
            RequestMessage::SetProbeId(r) => r.to_bytes(),
            RequestMessage::SetProbeColor(r) => r.to_bytes(),
            RequestMessage::ReadSessionInformation(r) => r.to_bytes(),
            RequestMessage::ReadLogs(r) => r.to_bytes(),
            RequestMessage::ProbeStatusMessage(_) => {
                Err(DekuError::Unexpected("Not implimented".to_string()))
            }
            RequestMessage::HeartbeatMessage(r) => r.to_bytes(),
            RequestMessage::SyncThermometerList(r) => r.to_bytes(),
        }
    }

    // This could just return Request::new(self), but we're using it to make sure we've implemented
    // EncapsulatableMessage for all RequestMessage variants.
    pub fn encapsulate(self) -> Request {
        match self {
            RequestMessage::SetProbeId(r) => r.encapsulate(),
            RequestMessage::SetProbeColor(r) => r.encapsulate(),
            RequestMessage::ReadSessionInformation(r) => r.encapsulate(),
            RequestMessage::ReadLogs(r) => r.encapsulate(),
            RequestMessage::ProbeStatusMessage(r) => r.encapsulate(),
            RequestMessage::HeartbeatMessage(r) => r.encapsulate(),
            RequestMessage::SyncThermometerList(r) => r.encapsulate(),
        }
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(magic = b"\xca\xfe")]
pub struct RequestHeader {
    crc: u16,
    #[deku(assert = "*request_type >> 7 == 0")]
    request_type: u8,
    #[deku(endian = "little")]
    pub request_id: u32,
    payload_length: u8,
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct Request {
    pub request_header: RequestHeader,
    #[deku(ctx = "request_header.request_type")]
    pub message: RequestMessage,
}

impl Request {
    pub fn new(message: RequestMessage) -> Self {
        Request::new_with_id(message, rand::random())
    }

    fn new_with_id(message: RequestMessage, request_id: u32) -> Self {
        let binding = Crc::<u16>::new(&CRC_16_IBM_3740);
        let mut digest = binding.digest();

        let message_type_id = message
            .deku_id()
            .expect("New message doesn't have Deku id.");

        let message_bytes = message.to_bytes().unwrap();

        // CRC of message type, request ID, payload length, and payload bytes.
        // TODO: when implementing response messages, this will need to be updated:
        // CRC of message type, request ID, response ID, success, payload length, and payload bytes
        digest.update(&[message_type_id]);
        digest.update(request_id.to_le_bytes().as_slice());
        digest.update(&[message_bytes.len() as u8]);
        digest.update(&message_bytes);

        let request_header = RequestHeader {
            crc: digest.finalize(),
            request_type: message_type_id,
            payload_length: message_bytes.len() as u8,
            request_id,
        };

        Self {
            request_header,
            message,
        }
    }
}

// Test that NodeMessages can be converted from a request to bytes
#[test]
fn test_heartbeat_message_to_bytes() {
    let heartbeat_message = RequestMessage::HeartbeatMessage(HeartbeatMessage {
        node_serial_number: [84, 49, 48, 48, 48, 48, 48, 51, 75, 86],
        mac_address: MacAddress {
            address: [0xc1, 0x88, 0x0b, 0xca, 0x6e, 0x81],
        },
        product_type: ProductType::MeatNetRepeater,
        hop_count: 0,
        is_inbound: Direction::Inbound,
        connection_details: [
            ConnectionDetailRecord {
                serial_number: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                product_type: ProductType::Unknown,
                attributes: Attributes {
                    connection_detail_record_is_populated: false,
                },
                rssi: 0,
            },
            ConnectionDetailRecord {
                serial_number: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                product_type: ProductType::MeatNetRepeater,
                attributes: Attributes {
                    connection_detail_record_is_populated: true,
                },
                rssi: 199,
            },
            ConnectionDetailRecord {
                serial_number: [57, 15, 2, 0, 0, 0, 0, 0, 0, 0],
                product_type: ProductType::MeatNetRepeater,
                attributes: Attributes {
                    connection_detail_record_is_populated: true,
                },
                rssi: 211,
            },
            ConnectionDetailRecord {
                serial_number: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                product_type: ProductType::Unknown,
                attributes: Attributes {
                    connection_detail_record_is_populated: false,
                },
                rssi: 0,
            },
        ],
    });

    let nm = Request::new_with_id(heartbeat_message, 0xa850cd42);
    nm.to_bytes()
        .unwrap()
        .iter()
        .for_each(|b| print!("{:02x} ", b));

    assert_eq!(
        nm.to_bytes().unwrap(),
        vec![
            0xca, 0xfe, 0x76, 0x44, 0x49, 0x42, 0xcd, 0x50, 0xa8, 0x47, 0x54, 0x31, 0x30, 0x30,
            0x30, 0x30, 0x30, 0x33, 0x4b, 0x56, 0xc1, 0x88, 0x0b, 0xca, 0x6e, 0x81, 0x02, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01, 0xc7, 0x39,
            0x0f, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01, 0xd3, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
        ]
    );
}

#[test]
fn test_session_information_message_to_bytes() {
    let read_session_information = RequestMessage::ReadSessionInformation(ReadSessionInformation {
        serial_number: SerialNumber { number: 0x10001DED },
    });

    let nm = Request::new_with_id(read_session_information, 0xa850cd42);

    let expected = vec![
        0xca, 0xfe, 0xe9, 0xb5, 0x03, 0x42, 0xcd, 0x50, 0xa8, 0x04, 0xed, 0x1d, 0x00, 0x10,
    ];

    assert_eq!(nm.to_bytes().unwrap(), expected)
}
