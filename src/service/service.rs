use std::net::AddrParseError;
use std::net::IpAddr;
use std::net::SocketAddr;

use super::ServiceLocator;
use config::Config;
use config::Environment;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use tracing::Level;

use super::Error;

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
  pub name: String,
  pub address: String,
  pub external_ip: IpAddr,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LogConfig {}

#[derive(Clone, Debug, Deserialize)]
pub struct ServiceConfig {
  pub server: ServerConfig,
  pub log: LogConfig,
}

pub struct Service {
  config: Config,
  service_config: ServiceConfig,
  service_locator: ServiceLocator,
}

#[derive(Default)]
pub struct ServiceOptions {}

impl Service {
  pub fn try_new(opts: ServiceOptions) -> Result<Self, anyhow::Error> {
    let config = Self::init_config(&opts)?;
    let service_locator = ServiceLocator::try_new(Self::init_service_locator(&opts)?)?;
    let service_config: ServiceConfig = config.clone().try_deserialize()?;

    let _ = Self::init_logging(&opts, &service_config.log)?;

    let svc = Service {
      config,
      service_config,
      service_locator,
    };

    Ok(svc)
  }

  fn init_config(_opts: &ServiceOptions) -> Result<Config, config::ConfigError> {
    Config::builder()
      .add_source(config::File::with_name("config/application"))
      .add_source(Environment::with_prefix("APP").separator("_"))
      .build()
  }

  fn init_service_locator(_opts: &ServiceOptions) -> Result<Config, config::ConfigError> {
    Config::builder()
      .add_source(config::File::with_name("config/service"))
      .add_source(Environment::with_prefix("SVC").separator("_"))
      .build()
  }

  fn init_logging(_opts: &ServiceOptions, _config: &LogConfig) -> Result<(), anyhow::Error> {
    std::env::set_var("RUST_LOG", "debug,kube=debug");

    tracing_subscriber::fmt()
      .json()
      .flatten_event(true)
      .with_max_level(Level::INFO)
      .init();

    Ok(())
  }

  pub fn config<T: DeserializeOwned>(&self) -> Result<T, Error> {
    self.config.clone().try_deserialize().map_err(|error| {
      tracing::error!(message = "Failed to read config", %error);
      error.into()
    })
  }

  pub fn service_locator(&self) -> &ServiceLocator {
    &self.service_locator
  }

  pub fn address(&self) -> Result<SocketAddr, Error> {
    self
      .service_config
      .server
      .address
      .parse()
      .map_err(|error: AddrParseError| {
        tracing::error!(message = "Failed to parse server address", %error);
        error.into()
      })
  }

  pub fn machine_id(&self) -> i64 {
    match self.service_config.server.external_ip {
      IpAddr::V4(ip) => (u32::from(ip) & 0x03FF) as i64,
      _ => panic!("Only ipv4 addesses supported"),
    }
  }
}
