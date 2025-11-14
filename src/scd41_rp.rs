use crate::calibration::CalibrationRegisters;
use crate::configuration::{SamplingConfiguration, SensorMode};
use crate::BME280Error::NotCalibrated;
use crate::{
    BME280Error, BME280Response, BME280_REGISTER_CHIPID,
    BME280_REGISTER_CONFIG, BME280_REGISTER_CONTROL, BME280_REGISTER_CONTROLHUMID,
    BME280_REGISTER_DATA_LENGTH, BME280_REGISTER_DATA_START, BME280_REGISTER_DIG_FIRST_LENGTH,
    BME280_REGISTER_DIG_SECOND_LENGTH, BME280_REGISTER_SOFTRESET, BME280_REGISTER_STATUS,
};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal_async::i2c::I2c;

pub struct BME280Sensor<'a, T: I2c> {
    i2c: &'a mut T,
    address: u8,
    calibration_registers: Option<CalibrationRegisters>,
}

impl<'a, T: I2c> BME280Sensor<'a, T> {
    pub fn new(i2c: &'a mut T, address: u8) -> Self {
        Self {
            i2c,
            address,
            calibration_registers: None,
        }
    }

    pub async fn setup(
        &mut self,
        sampling_configuration: SamplingConfiguration,
    ) -> Result<(), BME280Error> {
        let chip_id = self.read_register_u8(BME280_REGISTER_CHIPID).await?;
        if chip_id != 0x60 {
            return Err(BME280Error::InvalidChipId(chip_id));
        }
        self.write_register_8u(BME280_REGISTER_SOFTRESET, 0x86)
            .await?;
        Timer::after(Duration::from_millis(10)).await;
        let timeout = with_timeout(Duration::from_secs(1), async {
            loop {
                match self.is_reading_calibration().await {
                    Ok(reading) => {
                        if reading {
                            Timer::after(Duration::from_millis(10)).await;
                        } else {
                            break;
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        })
        .await;
        if let Err(_) = timeout {
            return Err(BME280Error::Timeout);
        }

        self.read_coefficients().await?;
        self.set_sampling_configuration(sampling_configuration)
            .await?;
        Timer::after(embassy_time::Duration::from_millis(100)).await;
        Ok(())
    }

    async fn is_reading_calibration(&mut self) -> Result<bool, BME280Error> {
        let status = self.read_register_u8(BME280_REGISTER_STATUS).await?;
        Ok((status & (1 << 3)) != 0)
    }

    async fn read_coefficients(&mut self) -> Result<(), BME280Error> {
        let mut data = [0u8; BME280_REGISTER_DIG_FIRST_LENGTH + BME280_REGISTER_DIG_SECOND_LENGTH];
        self.read_registers_bulk(0x88, &mut data[0..BME280_REGISTER_DIG_FIRST_LENGTH])
            .await?;
        self.read_registers_bulk(
            0xE1,
            &mut data[BME280_REGISTER_DIG_FIRST_LENGTH
                ..BME280_REGISTER_DIG_FIRST_LENGTH + BME280_REGISTER_DIG_SECOND_LENGTH],
        )
        .await?;

        self.calibration_registers = Some(data.into());

        Ok(())
    }

    async fn set_sampling_configuration(
        &mut self,
        sampling_configuration: SamplingConfiguration,
    ) -> Result<(), BME280Error> {
        let (config, ctrl_meas, ctrl_hum) = sampling_configuration.to_low_level_configuration();

        self.write_register_8u(BME280_REGISTER_CONTROL, SensorMode::Sleep as u8)
            .await?;
        self.write_register_8u(BME280_REGISTER_CONTROLHUMID, ctrl_hum.into())
            .await?;
        self.write_register_8u(BME280_REGISTER_CONFIG, config.into())
            .await?;
        self.write_register_8u(BME280_REGISTER_CONTROL, ctrl_meas.into())
            .await?;
        Ok(())
    }

    pub async fn read(&mut self) -> Result<BME280Response, BME280Error> {
        let mut data: [u8; BME280_REGISTER_DATA_LENGTH] = [0; BME280_REGISTER_DATA_LENGTH];
        self.read_registers_bulk(BME280_REGISTER_DATA_START, &mut data)
            .await?;

        let data_msb = (data[0] as u32) << 12;
        let data_lsb = (data[1] as u32) << 4;
        let data_xlsb = (data[2] as u32) >> 4;
        let adc_p = data_msb | data_lsb | data_xlsb;

        let data_msb = (data[3] as u32) << 12;
        let data_lsb = (data[4] as u32) << 4;
        let data_xlsb = (data[5] as u32) >> 4;
        let adc_t = (data_msb | data_lsb | data_xlsb) as i32;

        let data_msb = (data[6] as u32) << 8;
        let data_lsb = data[7] as u32;
        let adc_h = data_msb | data_lsb;

        if let Some(cr) = &self.calibration_registers {
            let t_fine = cr.compensate_temperature(adc_t);
            let temperature = ((t_fine * 5 + 128) >> 8) as f32 / 100.0;
            let humidity = cr.compensate_humidity(adc_h as u16, t_fine) as f32 / 1024.0;
            let pressure = cr.compensate_pressure(adc_p, t_fine) as f32 / 256.0;

            Ok(BME280Response {
                temperature,
                humidity,
                pressure,
            })
        } else {
            Err(NotCalibrated)
        }
    }

    async fn read_register_u8(&mut self, register: u8) -> Result<u8, BME280Error> {
        let mut buf = [0u8; 1];
        self.i2c_write_read(&[register], &mut buf).await?;
        Ok(buf[0])
    }

    async fn write_register_8u(&mut self, register: u8, data: u8) -> Result<(), BME280Error> {
        self.i2c_write(&[register, data]).await?;
        Ok(())
    }

    async fn read_registers_bulk(
        &mut self,
        register: u8,
        read: &mut [u8],
    ) -> Result<(), BME280Error> {
        self.i2c_write_read(&[register], read).await?;
        Ok(())
    }

    async fn i2c_write_read(&mut self, write: &[u8], read: &mut [u8]) -> Result<(), BME280Error> {
        match self.i2c.write_read(self.address, write, read).await {
            Ok(_) => Ok(()),
            Err(_) => Err(BME280Error::I2CError),
        }
    }

    async fn i2c_write(&mut self, write: &[u8]) -> Result<(), BME280Error> {
        match self.i2c.write(self.address, write).await {
            Ok(_) => Ok(()),
            Err(_) => Err(BME280Error::I2CError),
        }
    }
}
