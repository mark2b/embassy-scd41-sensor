#![no_std]
#![no_main]

mod scd41_rp;

pub use scd41_rp::SCD41Sensor;

#[derive(Clone)]
pub struct SCD41Response {
    pub co2: f32,
    pub humidity: f32,
    pub temperature: f32,
}

#[derive(Debug, Clone)]
pub enum SCD41Error {
    NoData,
    I2CError,
    Timeout,
}

