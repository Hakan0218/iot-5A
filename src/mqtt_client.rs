use anyhow::Result;
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration, QoS, EventPayload};
use std::time::Duration;

pub const THINGSBOARD_HOST: &str = "mqtt://thingsboard.cloud:1883";
pub const ACCESS_TOKEN: &str = "mdMPaSQQVC9rjPiEBLvN";

pub fn connect_thingsboard() -> Result<EspMqttClient<'static>> {
    let conf = MqttClientConfiguration {
        client_id: Some("esp32-s3-device"),
        username: Some(ACCESS_TOKEN),
        keep_alive_interval: Some(Duration::from_secs(30)),
        ..Default::default()
    };

    let (mut client, mut connection) = EspMqttClient::new(THINGSBOARD_HOST, &conf)?;

    std::thread::spawn(move || {
        while let Ok(event) = connection.next() {
            if let EventPayload::Received { topic, data, .. } = event.payload() {
                log::info!("Pesan masuk dari topic {:?}: {:?}", topic, String::from_utf8_lossy(data));
            }
        }
    });

    client.subscribe("v1/devices/me/rpc/request/+", QoS::AtLeastOnce)?;
    client.subscribe("v1/devices/me/attributes", QoS::AtLeastOnce)?;

    Ok(client)
}

pub fn send_telemetry(client: &mut EspMqttClient<'static>, temperature: f32, humidity: f32) -> Result<()> {
    let payload = serde_json::json!({
        "temperature": temperature,
        "humidity": humidity
    });
    client.publish(
        "v1/devices/me/telemetry",
        QoS::AtLeastOnce,
        false,
        payload.to_string().as_bytes(),
    )?;
    Ok(())
}