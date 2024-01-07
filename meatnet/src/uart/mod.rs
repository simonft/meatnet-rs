pub mod request;
pub mod response;

use core::panic;

use deku::prelude::*;

use crate::{MacAddress, ProductType};

use request::Request;
use response::Response;

use crc::{Crc, CRC_16_IBM_3740};

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(ctx = "message_type_id: u8", id = "message_type_id")]
pub enum MessageType {
    #[deku(id = "0")]
    Request(Request),
    #[deku(id = "1")]
    Response(Response),
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(magic = b"\xca\xfe")]
pub struct NodeMessage {
    crc: u16,
    #[deku(bits = "1")]
    message_type_id: u8,
    #[deku(ctx = "*message_type_id")]
    pub message_type: MessageType,
}

impl NodeMessage {
    pub fn new(message_type: MessageType) -> Self {
        let binding = Crc::<u16>::new(&CRC_16_IBM_3740);
        let mut digest = binding.digest();

        let (request_id, request_response_type, message_bytes) = match &message_type {
            MessageType::Request(r) => (
                r.request_id,
                r.message
                    .deku_id()
                    .expect("New message doesn't have Deku id."),
                r.message.to_bytes().unwrap(),
            ),
            MessageType::Response(_) => panic!("Response message write not implemented."),
        };

        let message_type_id = message_type
            .deku_id()
            .expect("New message doesn't have Deku id.");

        // CRC of message type, request ID, payload length, and payload bytes.
        // TODO: when implementing response messages, this will need to be updated:
        // CRC of message type, request ID, response ID, success, payload length, and payload bytes
        digest.update(&[request_response_type + 128 * message_type_id]);
        digest.update(request_id.to_le_bytes().as_slice());
        digest.update(&[message_bytes.len() as u8]);
        digest.update(&message_bytes);

        Self {
            crc: digest.finalize(),
            message_type_id,
            message_type,
        }
    }
}

// Test that NodeMessages can be converted from a request to bytes
#[test]
fn test_heartbeat_message_to_bytes() {
    let heartbeat_message = request::RequestType::HeartbeatMessage(request::HeartbeatMessage {
        node_serial_number: [84, 49, 48, 48, 48, 48, 48, 51, 75, 86],
        mac_address: MacAddress {
            address: [0xc1, 0x88, 0x0b, 0xca, 0x6e, 0x81],
        },
        product_type: ProductType::MeatNetRepeater,
        hop_count: 0,
        is_inbound: request::Direction::Inbound,
        connection_details: [
            request::ConnectionDetailRecord {
                serial_number: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                product_type: ProductType::Unknown,
                attributes: request::Attributes {
                    connection_detail_record_is_populated: false,
                },
                rssi: 0,
            },
            request::ConnectionDetailRecord {
                serial_number: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                product_type: ProductType::MeatNetRepeater,
                attributes: request::Attributes {
                    connection_detail_record_is_populated: true,
                },
                rssi: 199,
            },
            request::ConnectionDetailRecord {
                serial_number: [57, 15, 2, 0, 0, 0, 0, 0, 0, 0],
                product_type: ProductType::MeatNetRepeater,
                attributes: request::Attributes {
                    connection_detail_record_is_populated: true,
                },
                rssi: 211,
            },
            request::ConnectionDetailRecord {
                serial_number: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                product_type: ProductType::Unknown,
                attributes: request::Attributes {
                    connection_detail_record_is_populated: false,
                },
                rssi: 0,
            },
        ],
    });

    let nm = NodeMessage::new(MessageType::Request(Request::new_with_id(
        heartbeat_message,
        0xa850cd42,
    )));
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
    let read_session_information =
        request::RequestType::ReadSessionInformation(request::ReadSessionInformation {
            serial_number: super::ProbeSerialNumber { number: 0x10001DED },
        });

    let nm = NodeMessage::new(MessageType::Request(Request::new_with_id(
        read_session_information,
        0xa850cd42,
    )));

    let expected = vec![
        0xca, 0xfe, 0xe9, 0xb5, 0x03, 0x42, 0xcd, 0x50, 0xa8, 0x04, 0xed, 0x1d, 0x00, 0x10,
    ];

    assert_eq!(nm.to_bytes().unwrap(), expected,)
}
