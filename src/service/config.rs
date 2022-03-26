use serde_derive::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct ServiceInstance {
  pub address: String,
  pub shard_ranges: Vec<(String, String)>  
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServiceConf {
  pub name: String,
  pub instances: Vec<ServiceInstance>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
  pub cluster: ServiceConf,
  pub version: i64,
}
