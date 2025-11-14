#![no_std]
#![no_main]

use defmt::{error, info};
use defmt_rtt as _;
use embassy_bme280_sensor::bme280_rp::BME280Sensor;
use embassy_bme280_sensor::configuration::{Filter, Oversampling, SamplingConfiguration, SensorMode, StandbyDuration};
use embassy_bme280_sensor::BME280Error;
use embassy_executor::Spawner;
use embassy_rp::peripherals::I2C0;
use embassy_rp::{bind_interrupts, i2c};
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
    let mut sensor = BME280Sensor::new(&mut i2c, 0x76);

    // Configure and initialize sensor
    match sensor
        .setup(
            SamplingConfiguration::default()
                .with_temperature_oversampling(Oversampling::X4)
                .with_pressure_oversampling(Oversampling::X4)
                .with_humidity_oversampling(Oversampling::X4)
                .with_sensor_mode(SensorMode::Normal)
                .with_standby_duration(StandbyDuration::Millis1000)
                .with_filter(Filter::X8),

        )
        .await {
        Ok(_) => info!("BME280 sensor initialized successfully"),
        Err(e) => {
            error!("Failed to initialize BME280 sensor: ");
        }
    }

    // Read sensor data
    loop {
        match sensor.read().await {
            Ok(data) => {
                info!(
                    "Temperature: {}°C, Humidity: {}%, Pressure: {} Pa",
                    data.temperature, data.humidity, data.pressure
                );
            }
            Err(e) => match e {
                BME280Error::NoData => error!("No data"),
                BME280Error::I2CError => error!("I2C communication error"),
                BME280Error::InvalidChipId(id) => error!("Invalid chip ID: {}", id),
                BME280Error::Timeout => error!("Operation timed out"),
                _ => error!("Other error"),
            },
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}
