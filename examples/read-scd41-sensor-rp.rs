#![no_std]
#![no_main]

use defmt::{error, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::peripherals::I2C0;
use embassy_rp::{bind_interrupts, i2c};
use embassy_scd41_sensor::{SCD41Error, SCD41Sensor};
use embassy_time::{Duration, Timer};
use panic_probe as _;

bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());

    let sda = p.PIN_0;
    let scl = p.PIN_1;

    // Configure I2C
    let mut i2c = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, Default::default());

    // Create sensor instance
    let mut sensor = SCD41Sensor::new(&mut i2c, 0x62);

    // Read sensor data
    loop {
        match sensor.read().await {
            Ok(data) => {
                info!(
                    "Temperature: {}°C, Humidity: {}%, CO2: {}",
                    data.temperature, data.humidity, data.co2
                );
            }
            Err(e) => match e {
                SCD41Error::NoData => error!("No data"),
                SCD41Error::I2CError => error!("I2C communication error"),
                SCD41Error::Timeout => error!("Operation timed out"),
            },
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}
