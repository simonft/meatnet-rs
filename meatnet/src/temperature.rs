use deku::prelude::*;

pub trait IsTemperature {
    fn get_celsius(&self) -> f32;

    fn get_fahrenheit(&self) -> f32 {
        (self.get_celsius() * 9.0 / 5.0) + 32.0
    }
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct Temperature {
    raw_value: u16,
}

impl Temperature {
    pub fn new(raw_value: u16) -> Self {
        Temperature { raw_value }
    }

    pub fn get_raw_value(&self) -> u16 {
        self.raw_value
    }
}

impl IsTemperature for Temperature {
    fn get_celsius(&self) -> f32 {
        (self.raw_value as f32 * 0.05) - 20.0
    }

    fn get_fahrenheit(&self) -> f32 {
        (self.get_celsius() * 9.0 / 5.0) + 32.0
    }
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct CoreTemperature {
    #[deku(bits = "11", endian = "little")]
    raw_value: u16,
}

impl CoreTemperature {
    pub fn new(raw_value: u16) -> Self {
        Self { raw_value }
    }
}

impl IsTemperature for CoreTemperature {
    fn get_celsius(&self) -> f32 {
        (self.raw_value as f32 * 0.1) - 20.0
    }
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct PredictionSetPointTemperature {
    #[deku(bits = "10", endian = "little")]
    raw_value: u16,
}

impl PredictionSetPointTemperature {
    pub fn new(raw_value: u16) -> Self {
        Self { raw_value }
    }
}

impl IsTemperature for PredictionSetPointTemperature {
    fn get_celsius(&self) -> f32 {
        self.raw_value as f32 * 0.1
    }
}

#[derive(Debug, PartialEq, DekuRead)]
pub struct HeatStartTemperature {
    #[deku(bits = "10", endian = "little")]
    raw_value: u16,
}

impl HeatStartTemperature {
    pub fn new(raw_value: u16) -> Self {
        Self { raw_value }
    }
}

impl IsTemperature for HeatStartTemperature {
    fn get_celsius(&self) -> f32 {
        self.raw_value as f32 * 0.1
    }
}
