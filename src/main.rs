use anyhow::{bail, Result};
use core::str;
use embedded_svc::{http::client::Client, io::Read};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::client::{Configuration, EspHttpConnection},
};
mod wifi;
use wifi::wifi;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

use serde::{Deserialize, Serialize};

use semver::Version;

#[derive(Serialize, Deserialize, Debug)]
struct UpdateJson {
    version: String,
}

#[derive(Debug)]
struct Update {
    version: Version,
}

impl Update {
    pub fn new(json: UpdateJson) -> Update {
        let version = Version::parse(&json.version).unwrap();
        Update { version }
    }
}

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    // The constant `CONFIG` is auto-generated by `toml_config`.
    let app_config = CONFIG;

    // Connect to the Wi-Fi network
    let _wifi = wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop,
    )?;

    let update = check_update(
        "https://raw.githubusercontent.com/Mirkopoj/ESP-OTA-Template/master/update.json",
    )?;

    println!("{:?}", update);
    println!("{}", update.version);
    let cero = Version::parse("0.0.0").unwrap();
    println!("{}", update.version > cero);

    Ok(())
}

fn connect() -> Result<Client<EspHttpConnection>> {
    let connection = EspHttpConnection::new(&Configuration {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let client = Client::wrap(connection);

    Ok(client)
}

fn check_update(url: impl AsRef<str>) -> Result<Update> {

    let mut client = connect()?;
    let request = client.get(url.as_ref())?;
    let response = request.submit()?;
    let status = response.status();

    let update: Update;

    match status {
        200..=299 => {
            let mut buf = [0_u8; 256];
            let mut reader = response;
            let size = Read::read(&mut reader, &mut buf)?;
            if size == 0 {
                bail!("Zero sized message");
            }
            update = Update::new(serde_json::from_slice(&buf[..size])?);
        }
        _ => bail!("Unexpected response code: {}", status),
    }

    Ok(update)
}
