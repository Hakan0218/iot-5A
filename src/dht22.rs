use anyhow::Result;
use esp_idf_hal::delay::Delay;
use esp_idf_hal::gpio::{AnyIOPin, PinDriver, InputOutput};
use dht_hal_drv::{dht_read, DhtType};

pub fn read_dht22(driver: &mut PinDriver<'_, AnyIOPin, InputOutput>) -> Result<(f32, f32)> {
    let delay = Delay::new_default();

    match dht_read(DhtType::DHT22, driver, &mut |us| delay.delay_us(us.into())) {
        Ok(reading) => {
            let temperature = reading.temperature();
            let humidity = reading.humidity();
            Ok((temperature, humidity))
        }
        Err(e) => {
            anyhow::bail!("Gagal membaca DHT22: {:?}", e)
        }
    }
}