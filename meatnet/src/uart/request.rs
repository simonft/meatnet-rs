use deku::prelude::*;

use crate::{MacAddress, ProbeStatus, ProductType, SerialNumber};

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct SetProbeId {}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct SetProbeColor {}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct ReadSessionInformation {
    pub serial_number: SerialNumber,
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct ReadLogs {
    pub probe_serial_number: SerialNumber,
    pub sequence_number_start: u32,
    pub sequence_number_end: u32,
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

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(type = "u8")]
pub enum Hops {
    One = 0,
    Two,
    Three,
    Four,
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct NetworkInformation {
    pub hop_count: Hops,
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
    #[deku(id = "0x45")]
    ProbeStatusMessage(ProbeStatusMessage),
    #[deku(id = "0x49")]
    HeartbeatMessage(HeartbeatMessage),
    #[deku(id = "0x4b")]
    SyncThermometerList(SyncThermometerList),
}
impl RequestType {
    pub fn to_bytes(&self) -> Result<Vec<u8>, DekuError> {
        match self {
            RequestType::SetProbeId(r) => r.to_bytes(),
            RequestType::SetProbeColor(r) => r.to_bytes(),
            RequestType::ReadSessionInformation(r) => r.to_bytes(),
            RequestType::ReadLogs(r) => r.to_bytes(),
            RequestType::ProbeStatusMessage(_) => {
                Err(DekuError::Unexpected("Not implimented".to_string()))
            }
            RequestType::HeartbeatMessage(r) => r.to_bytes(),
            RequestType::SyncThermometerList(r) => r.to_bytes(),
        }
    }
}

#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
pub struct Request {
    #[deku(bits = "7")]
    pub request_type: u8,
    #[deku(endian = "little")]
    pub request_id: u32,
    payload_length: u8,
    #[deku(ctx = "*request_type")]
    pub message: RequestType,
}

impl Request {
    pub fn new(message: RequestType) -> Self {
        Request::new_with_id(message, rand::random())
    }

    pub fn new_with_id(message: RequestType, request_id: u32) -> Self {
        Self {
            request_type: message.deku_id().unwrap(),
            request_id,
            payload_length: message.to_bytes().unwrap().len() as u8,
            message,
        }
    }
}
