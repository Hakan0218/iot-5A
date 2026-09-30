use embedded_svc::io::Write;
use anyhow::Result;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, AccessPointConfiguration};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::http::server::{EspHttpServer, Configuration as HttpConfig};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WifiCred {
    pub ssid: String,
    pub password: String,
}

pub fn known_networks() -> Vec<WifiCred> {
    vec![
        WifiCred { ssid: "TripleNduts".into(),   password: "karepmudewe".into() },
        WifiCred { ssid: "HOSPOT_HP_SATRIA".into(),  password: "password2".into() },
        WifiCred { ssid: "Bismillah".into(), password: "12345678".into() },
    ]
}

pub fn connect_or_provision(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    creds: &[WifiCred],
) -> Result<()> {
    wifi.start()?;
    let scanned = wifi.scan()?;
    log::info!("Ditemukan {} jaringan WiFi di sekitar", scanned.len());

    for cred in creds {
        if scanned.iter().any(|ap| ap.ssid == cred.ssid.as_str()) {
            log::info!("Mencoba connect ke: {}", cred.ssid);
            let conf = Configuration::Client(ClientConfiguration {
                ssid: cred.ssid.as_str().try_into().unwrap(),
                password: cred.password.as_str().try_into().unwrap(),
                auth_method: AuthMethod::WPA2Personal,
                ..Default::default()
            });
            wifi.set_configuration(&conf)?;
            wifi.connect()?;

            if wifi.wait_netif_up().is_ok() {
                log::info!("Berhasil connect ke {}", cred.ssid);
                return Ok(());
            } else {
                log::warn!("Gagal connect ke {}, coba kandidat berikutnya", cred.ssid);
            }
        }
    }

    log::warn!("Tidak ada WiFi dikenal yang tersedia. Masuk mode konfigurasi (AP)...");
    start_provisioning_ap(wifi)
}

fn start_provisioning_ap(wifi: &mut BlockingWifi<EspWifi<'static>>) -> Result<()> {
    let ap_conf = Configuration::AccessPoint(AccessPointConfiguration {
        ssid: "ESP32-Setup".try_into().unwrap(),
        auth_method: AuthMethod::None,
        ..Default::default()
    });
    wifi.set_configuration(&ap_conf)?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    log::info!("AP 'ESP32-Setup' aktif. Connect HP Anda ke sini, lalu buka http://192.168.71.1");

    let new_ssid = Arc::new(Mutex::new(String::new()));
    let new_pass = Arc::new(Mutex::new(String::new()));
    let s1 = new_ssid.clone();
    let p1 = new_pass.clone();

    let mut server = EspHttpServer::new(&HttpConfig::default())?;
    server.fn_handler("/", esp_idf_svc::http::Method::Get, move |req| {
        let html = r#"<html><body>
            <h3>Setup WiFi ESP32</h3>
            <form action="/save" method="get">
              SSID: <input name="ssid"><br>
              Password: <input name="pass" type="password"><br>
              <button type="submit">Simpan</button>
            </form></body></html>"#;
        req.into_ok_response()?.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    server.fn_handler("/save", esp_idf_svc::http::Method::Get, move |req| {
        let uri = req.uri().to_string();
        if let Some(query) = uri.split('?').nth(1) {
            for pair in query.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    let decoded = v.replace('+', " ");
                    if k == "ssid" {
    *s1.lock().unwrap() = decoded;
} else if k == "pass" {
    *p1.lock().unwrap() = decoded;
}
                }
            }
        }
        req.into_ok_response()?
            .write_all(b"Tersimpan. ESP32 akan restart...")?;
        Ok::<(), anyhow::Error>(())
    })?;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let ssid = new_ssid.lock().unwrap().clone();
        if !ssid.is_empty() {
            let pass = new_pass.lock().unwrap().clone();
            save_wifi_to_nvs(&ssid, &pass)?;
            log::info!("WiFi baru disimpan: {}. Restarting...", ssid);
            unsafe { esp_idf_sys::esp_restart(); }
        }
    }
}

fn save_wifi_to_nvs(ssid: &str, pass: &str) -> Result<()> {
    use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
    let nvs_default_partition = EspDefaultNvsPartition::take()?;
    let mut nvs: EspNvs<NvsDefault> = EspNvs::new(nvs_default_partition, "wifi_cfg", true)?;
    nvs.set_str("ssid", ssid)?;
    nvs.set_str("pass", pass)?;
    Ok(())
}

pub fn load_saved_wifi() -> Option<WifiCred> {
    use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
    let nvs_default_partition = EspDefaultNvsPartition::take().ok()?;
    let nvs: EspNvs<NvsDefault> = EspNvs::new(nvs_default_partition, "wifi_cfg", true).ok()?;
    let mut ssid_buf = [0u8; 32];
    let mut pass_buf = [0u8; 64];
    let ssid = nvs.get_str("ssid", &mut ssid_buf).ok().flatten()?;
    let pass = nvs.get_str("pass", &mut pass_buf).ok().flatten()?;
    Some(WifiCred { ssid: ssid.to_string(), password: pass.to_string() })
}