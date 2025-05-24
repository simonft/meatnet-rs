use alloc::format;
use deku::prelude::*;
use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(id_type = "u8")]
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

#[repr(u8)]
#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(bits = "2", id_type = "u8")]
pub enum PredictionMode {
    None = 0,
    TimeToRemoval,
    RemovalAndResting,
    Reserved,
}

#[repr(u8)]
#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(bits = "2", id_type = "u8")]
pub enum PredictionType {
    None = 0,
    Removal,
    Resting,
    Reserved,
}

#[repr(u8)]
#[derive(Debug, PartialEq, DekuWrite, DekuRead)]
#[deku(bits = "4", id_type = "u8")]
pub enum PredictionState {
    ProbeNotInserted = 0,
    ProbeInserted,
    Warming,
    Predicting,
    RemovalPredictionDone,
    ReservedState5,
    ReservedState6,
    ReservedState7,
    ReservedState8,
    ReservedState9,
    ReservedState10,
    ReservedState11,
    ReservedState12,
    ReservedState13,
    ReservedState14,
    Unknown,
}

#[repr(u8)]
#[derive(Debug, PartialEq, DekuRead, Clone)]
#[deku(id_type = "u8", bits = "2")]
pub enum Mode {
    Normal = 0,
    InstantRead,
    Reserved,
    Errored,
}

#[repr(u8)]
#[derive(Debug, PartialEq, DekuRead)]
#[deku(id_type = "u8", bits = "3")]
pub enum Color {
    Yellow = 0,
    Grey,
    Reserved2,
    Reserved3,
    Reserved4,
    Reserved5,
    Reserved6,
    Reserved7,
}

#[repr(u8)]
#[derive(Debug, PartialEq, DekuRead)]
#[deku(id_type = "u8", bits = "1")]
pub enum BatteryStatus {
    Ok = 0,
    LowBattery,
}

#[repr(u8)]
#[derive(Debug, PartialEq, DekuRead, DekuWrite, Clone)]
#[deku(id_type = "u8")]
pub enum ProductType {
    Unknown = 0,
    PredictiveProbe,
    MeatNetRepeater,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite, Clone, Copy, Serialize, Deserialize)]
#[deku(endian = "little")]
pub struct SerialNumber {
    pub number: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct MacAddress {
    pub address: [u8; 6],
}
