use crate::proto::longrunning::operations_client::OperationsClient;
use crate::proto::system::clusters_client::ClustersClient;
use crate::service::ClusterSvcClient;
use crate::service::ClusterWorkspacesClient;
use crate::service::OperationsSvcClient;

use super::client::ShardedClient;

use serde_derive::Deserialize;

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ServiceInstance {
  pub address: String,
  pub shard_ranges: Vec<(String, String)>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ServiceConf {
  pub name: String,
  pub instances: Vec<ServiceInstance>,
}

#[allow(dead_code, unused)]
#[derive(Clone, Debug, Deserialize)]
struct LocatorConfig {
  pub cluster: ServiceConf,
  pub longrunning: ServiceConf,
  pub system: ServiceConf,
  pub version: i64,
}

#[async_trait::async_trait]
pub trait ServiceRegistry<T: Clone> {
  async fn get(&self) -> anyhow::Result<ShardedClient<T>>;
}

#[derive(Debug, Clone)]
pub struct ServiceLocator {
  clusters: ShardedClient<ClusterSvcClient>,
  operations: ShardedClient<OperationsSvcClient>,
  cluster_workspaces: ShardedClient<ClusterWorkspacesClient>,
}

impl ServiceLocator {
  pub fn try_new(conf: config::Config) -> anyhow::Result<ServiceLocator> {
    let config: LocatorConfig = conf.try_deserialize()?;

    Ok(ServiceLocator {
      clusters: ShardedClient::try_new(config.system, ClustersClient::new)?,
      operations: ShardedClient::try_new(config.longrunning, OperationsClient::new)?,
      cluster_workspaces: ShardedClient::try_new(config.cluster, ClusterWorkspacesClient::new)?,
    })
  }
}

#[async_trait::async_trait]
impl ServiceRegistry<ClusterSvcClient> for ServiceLocator {
  async fn get(&self) -> anyhow::Result<ShardedClient<ClusterSvcClient>> {
    Ok(self.clusters.clone())
  }
}

#[async_trait::async_trait]
impl ServiceRegistry<ClusterWorkspacesClient> for ServiceLocator {
  async fn get(&self) -> anyhow::Result<ShardedClient<ClusterWorkspacesClient>> {
    Ok(self.cluster_workspaces.clone())
  }
}

#[async_trait::async_trait]
impl ServiceRegistry<OperationsSvcClient> for ServiceLocator {
  async fn get(&self) -> anyhow::Result<ShardedClient<OperationsSvcClient>> {
    Ok(self.operations.clone())
  }
}
