use defmt::{info, println};
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;
use crate::{SCD41Error, SCD41Response};

pub struct SCD41Sensor<'a, T: I2c> {
    i2c: &'a mut T,
    address: u8,
    initialized: bool,
    initialization_step: InitializationStep,
    last_response: Option<SCD41Response>,
    last_read_time: Option<embassy_time::Instant>,
    next_step_time: embassy_time::Instant,
}

impl<'a, T: I2c> SCD41Sensor<'a, T> {
    pub fn new(i2c: &'a mut T, address: u8) -> Self {
        Self {
            i2c,
            address,
            initialized: false,
            initialization_step: InitializationStep::Initial,
            last_response : None,
            last_read_time : None,
            next_step_time : embassy_time::Instant::now(),
        }
    }

    pub async fn read(&mut self) -> Result<SCD41Response, SCD41Error> {

        let now = embassy_time::Instant::now();
        // if let Some(last_read_time) = self.last_read_time {
        //     if now - last_read_time < Duration::from_secs(crate::dht::MIN_REQUEST_INTERVAL_SECS) {
        //         if let Some(response) = &self.last_response {
        //             return Ok(response.clone());
        //         }
        //     }
        // }
        // else {
        //     if now.as_secs() < crate::dht::MIN_REQUEST_INTERVAL_SECS {
        //         return Err(DHTSensorError::NoData);
        //     }
        // }
        if now < self.next_step_time  {
            return if let Some(response) = &self.last_response {
                Ok(response.clone())
            } else {
                Err(SCD41Error::NoData)
            }
        }
        match self.initialization_step {
            InitializationStep::Initial => {
                println!("Initialization Step: Initial -> StopMeasurement");
                self.initialization_step = InitializationStep::StopMeasurement;
                self.next_step_time = now + Duration::from_millis(1000);
                Err(SCD41Error::NoData)
            }
            InitializationStep::StopMeasurement => {
                println!("Initialization Step: StopMeasurement -> Reinit");
                self.initialization_step = InitializationStep::Reinit;
                self.next_step_time = now + Duration::from_millis(1000);
                self.i2c_write(&[0x3f, 0x86]).await?;
                Err(SCD41Error::NoData)
            }
            InitializationStep::Reinit => {
                println!("Initialization Step: Reinit -> StartMeasurement");
                self.initialization_step = InitializationStep::StartMeasurement;
                self.next_step_time = now + Duration::from_millis(1000);
                self.i2c_write(&[0x36, 0x46]).await?;
                Err(SCD41Error::NoData)
            }
            InitializationStep::StartMeasurement => {
                println!("Initialization Step: StartMeasurement -> ReadData");
                self.initialization_step = InitializationStep::ReadData;
                self.next_step_time = now + Duration::from_millis(1000);
                self.i2c_write(&[0x21, 0xb1]).await?;
                Err(SCD41Error::NoData)
            }
            InitializationStep::ReadData => {
                println!("Initialization Step: ReadData");
                self.next_step_time = now + Duration::from_millis(5000);
                let mut buf = [0u8; 9];
                info!("Read I2C status");
                self.i2c_write_read(&[0xe4, 0xb8], &mut buf).await?;
                if buf[1] == 0x0 {
                    self.next_step_time = now + Duration::from_millis(1000);
                    info!("Data not ready, waiting...");
                    Err(SCD41Error::NoData)
                }
                else {
                    self.i2c_write_read(&[0xec, 0x05], &mut buf).await?;
                    let delimiter = 0xffff as f32;
                    info!("Received I2C data: {:?}", &buf);
                    let co2_ppm = i16::from_be_bytes([buf[0], buf[1]]) as f32;
                    let temperature_data = i16::from_be_bytes([buf[3], buf[4]]) as f32;
                    let temperature = -45f32 + 175f32 * temperature_data / delimiter;
                    let humidity_data = i16::from_be_bytes([buf[6], buf[7]]) as f32;
                    let humidity = 100f32 * (humidity_data / delimiter);
                    let response = SCD41Response {
                        co2: co2_ppm,
                        temperature,
                        humidity,
                    };
                    self.last_response = Some(response.clone());
                    Ok(response)
                }
            }
        }
    }

    async fn i2c_write_read(&mut self, write: &[u8], read: &mut [u8]) -> Result<(), SCD41Error> {
        match self.i2c.write_read(self.address, write, read).await {
            Ok(_) => Ok(()),
            Err(_) => Err(SCD41Error::I2CError),
        }
    }

    async fn i2c_write(&mut self, write: &[u8]) -> Result<(), SCD41Error> {
        match self.i2c.write(self.address, write).await {
            Ok(_) => Ok(()),
            Err(_) => Err(SCD41Error::I2CError),
        }
    }
}


enum InitializationStep {
    Initial,
    StopMeasurement,
    Reinit,
    StartMeasurement,
    ReadData,
}


#[inline]
pub fn sensirion_crc8(data: &[u8]) -> u8 {
    const CRC8_POLYNOMIAL: u8 = 0x31;
    const CRC8_INIT: u8 = 0xFF;

    let mut crc: u8 = CRC8_INIT;

    for &b in data {
        crc ^= b;
        for _ in 0..8 {
            crc = if (crc & 0x80) != 0 {
                (crc << 1) ^ CRC8_POLYNOMIAL
            } else {
                crc << 1
            };
        }
    }

    crc
}
