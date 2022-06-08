use std::net::IpAddr;

use config::Config;
use config::Environment;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use tracing::Level;

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
  pub name: String,
  pub address: String,
  pub external_ip: IpAddr,
}

pub trait ServiceConfig {
    fn server_config(&self) -> &ServerConfig;
}

pub struct Service<C: ServiceConfig> {
  config: C,
}

#[derive(Default)]
pub struct ServiceOptions {}

impl<C: DeserializeOwned + ServiceConfig> Service<C> {
  pub fn try_new(opts: ServiceOptions) -> Result<Self, anyhow::Error> {
    let conf = Self::init_config(&opts)?;
    let config: C = conf.try_deserialize()?;

    let _ = Self::init_logging(&opts, &config)?;

    let svc = Service { config };

    Ok(svc)
  }

  fn init_config(_opts: &ServiceOptions) -> Result<Config, config::ConfigError> {
    Config::builder()
      .add_source(config::File::with_name("config/application"))
      .add_source(Environment::with_prefix("APP").separator("_"))
      .build()
  }

  fn init_logging(_opts: &ServiceOptions, _config: &C) -> Result<(), anyhow::Error> {
    std::env::set_var("RUST_LOG", "debug,kube=debug");

    tracing_subscriber::fmt()
      .json()
      .flatten_event(true)
      .with_max_level(Level::INFO)
      .init();

    Ok(())
  }

  pub fn config(&self) -> &C {
      &self.config
  }
}
