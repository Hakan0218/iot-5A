mod wifi_manager;
mod mqtt_client;
mod dht22;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::hal::peripherals::Peripherals;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // --- WiFi setup (seperti sebelumnya) ---
    let esp_wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(esp_wifi, sys_loop)?;

    let mut creds = wifi_manager::known_networks();
    if let Some(saved) = wifi_manager::load_saved_wifi() {
        creds.push(saved);
    }
    wifi_manager::connect_or_provision(&mut wifi, &creds)?;
    log::info!("WiFi tersambung, lanjut ke MQTT...");

    let mut mqtt_client = mqtt_client::connect_thingsboard()?;
    log::info!("Terhubung ke ThingsBoard!");

    // --- Setup pin DHT22 ---
        // --- Setup pin DHT22 (dibuat sekali di luar loop) ---
    let dht_pin: esp_idf_hal::gpio::AnyIOPin = peripherals.pins.gpio4.into();
    let mut dht_driver = esp_idf_hal::gpio::PinDriver::input_output_od(dht_pin)?;

    loop {
        match dht22::read_dht22(&mut dht_driver) {
            Ok((temperature, humidity)) => {
                mqtt_client::send_telemetry(&mut mqtt_client, temperature, humidity)?;
                log::info!("Data terkirim: temp={:.1}°C, hum={:.1}%", temperature, humidity);
            }
            Err(e) => {
                log::warn!("Gagal baca sensor: {:?}", e);
            }
        }

        std::thread::sleep(Duration::from_secs(5)); // DHT22 butuh jeda minimal 2 detik antar baca
    }
}