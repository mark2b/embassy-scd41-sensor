# embassy-bme280-sensor

[![Crates.io](https://img.shields.io/crates/v/embassy-bme280-sensor.svg)](https://crates.io/crates/embassy-bme280-sensor)
[![Documentation](https://docs.rs/embassy-bme280-sensor/badge.svg)](https://docs.rs/embassy-bme280-sensor)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/mark2b/embassy-bme280-sensor#license)

An async BME280 sensor driver for the [Embassy](https://embassy.dev/) async runtime, designed for embedded systems.

The BME280 is a combined digital humidity, pressure and temperature sensor based on proven sensing principles. This driver provides a high-level async interface for reading environmental data from BME280 sensors over I2C.

## Features

- **Async/await support** - Built for the Embassy async runtime
- **Comprehensive configuration** - Full control over oversampling, filtering, and sensor modes
- **Multiple platform support** - Currently supports RP2040, with extensible architecture
- **No-std compatible** - Designed for embedded systems
- **Type-safe configuration** - Builder pattern for sensor configuration
- **Automatic calibration** - Handles sensor calibration data reading and compensation
- **Error handling** - Comprehensive error types for robust applications

## Supported Platforms

- **RP2040** (Raspberry Pi Pico and compatible boards)

## Hardware Requirements

- BME280 sensor module
- I2C connection (SDA/SCL pins)
- Pull-up resistors on I2C lines (typically 4.7kΩ)

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
embassy-bme280-sensor = "0.1.0"
embassy-rp = "0.8.0"
embassy-executor = "0.9"
embassy-time = "0.5"
defmt = "1"
defmt-rtt = "1"
panic-probe = { version = "1", features = ["print-defmt"] }
```

### Basic Usage

```rust
#![no_std]
#![no_main]

use embassy_bme280_sensor::bme280_rp::BME280Sensor;
use embassy_bme280_sensor::configuration::{SamplingConfiguration, Oversampling, SensorMode, StandbyDuration};
use embassy_bme280_sensor::BME280Error;
use embassy_executor::Spawner;
use embassy_rp::peripherals::I2C0;
use embassy_rp::{bind_interrupts, i2c};
use embassy_time::{Duration, Timer};
use defmt::{error, info};
use defmt_rtt as _;
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
    sensor.setup(
        SamplingConfiguration::default()
            .with_temperature_oversampling(Oversampling::X1)
            .with_pressure_oversampling(Oversampling::X1)
            .with_humidity_oversampling(Oversampling::X1)
            .with_sensor_mode(SensorMode::Normal)
            .with_standby_duration(StandbyDuration::Millis1000)
    ).await.unwrap();
    
    // Read sensor data
    loop {
        match sensor.read().await {
            Ok(data) => {
                info!(
                    "Temperature: {:.2}°C, Humidity: {:.2}%, Pressure: {:.2} Pa",
                    data.temperature, data.humidity, data.pressure
                );
            }
            Err(e) => match e {
                BME280Error::ChecksumError => error!("Checksum error"),
                BME280Error::InvalidData => error!("Invalid data received from sensor"),
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
```

## Configuration Options

### Oversampling

Control the precision and power consumption of measurements:

```rust
use embassy_bme280_sensor::configuration::Oversampling;

// Available options:
Oversampling::Skip    // Skip measurement
Oversampling::X1      // 1x oversampling (fastest, lowest power)
Oversampling::X2      // 2x oversampling
Oversampling::X4      // 4x oversampling
Oversampling::X8      // 8x oversampling
Oversampling::X16     // 16x oversampling (most precise, highest power)
```

### Sensor Modes

```rust
use embassy_bme280_sensor::configuration::SensorMode;

SensorMode::Sleep     // Low power mode, no measurements
SensorMode::Forced    // Single measurement then sleep
SensorMode::Normal    // Continuous measurements
```

### Standby Duration

Control the interval between measurements in normal mode:

```rust
use embassy_bme280_sensor::configuration::StandbyDuration;

StandbyDuration::Millis0_5    // 0.5ms
StandbyDuration::Millis10     // 10ms
StandbyDuration::Millis20     // 20ms
StandbyDuration::Millis62_5   // 62.5ms
StandbyDuration::Millis125    // 125ms
StandbyDuration::Millis250    // 250ms
StandbyDuration::Millis500    // 500ms
StandbyDuration::Millis1000   // 1000ms
```

### Filtering

Apply digital filtering to reduce noise:

```rust
use embassy_bme280_sensor::configuration::Filter;

Filter::Off   // No filtering
Filter::X2    // 2x filtering
Filter::X4    // 4x filtering
Filter::X8    // 8x filtering
Filter::X16   // 16x filtering
```

## Data Structure

The sensor returns a `BME280Response` struct:

```rust
pub struct BME280Response {
    pub temperature: f32,  // Temperature in Celsius
    pub humidity: f32,     // Relative humidity in %
    pub pressure: f32,     // Pressure in Pascal
}
```

## Error Handling

The driver provides comprehensive error handling:

```rust
pub enum BME280Error {
    NoData,              // No data available
    I2CError,           // I2C communication error
    InvalidChipId(u8),  // Wrong chip ID detected
    Timeout,            // Operation timed out
    NotCalibrated,      // Sensor not properly calibrated
}
```

## I2C Address

The BME280 supports two I2C addresses:
- `0x76` (default, SDO pin connected to GND)
- `0x77` (SDO pin connected to VCC)

## Examples

See the `examples/` directory for complete working examples:

- `read-bme280-sensor-rp.rs` - Basic sensor reading example for RP2040

To run the example:

```bash
cargo run --example read-bme280-sensor-rp --features rp2040,examples
```

## Hardware Connections

### RP2040 (Raspberry Pi Pico)

| BME280 Pin | RP2040 Pin | Description |
|------------|------------|-------------|
| VCC        | 3.3V       | Power supply |
| GND        | GND        | Ground |
| SCL        | GP1        | I2C Clock |
| SDA        | GP0        | I2C Data |
| SDO        | GND        | I2C Address select (0x76) |

## Performance Considerations

- **Oversampling**: Higher oversampling provides better accuracy but increases measurement time and power consumption
- **Filtering**: Digital filtering reduces noise but adds latency
- **Standby duration**: Longer standby periods reduce power consumption in normal mode
- **Measurement time**: Typical measurement times range from 1ms (1x oversampling) to 20ms (16x oversampling)

## License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## Acknowledgments

- [Bosch Sensortec](https://www.bosch-sensortec.com/) for the BME280 sensor
- [Embassy](https://embassy.dev/) for the excellent async runtime
- The embedded Rust community for inspiration and support
