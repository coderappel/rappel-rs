use crate::proto::cluster::workspace_nodes_client::WorkspaceNodesClient;
use crate::proto::longrunning::operations_client::OperationsClient;
use crate::proto::system::clusters_client::ClustersClient;
use crate::service::ClusterSvcClient;
use crate::service::OperationsSvcClient;
use crate::service::WorkspaceNodesSvcClient;

use super::client::ShardedClient;

#[derive(Debug, Clone)]
pub struct ServiceLocator {
  clusters: ShardedClient<ClusterSvcClient>,
  operations: ShardedClient<OperationsSvcClient>,
  workspace_nodes: ShardedClient<WorkspaceNodesSvcClient>,
}

const ERROR_MISSING_SERVICE: &str = "Missing Service";

impl ServiceLocator {
  pub async fn try_new() -> Result<ServiceLocator, super::Error> {
    let conf = config::Config::builder()
      .add_source(config::File::with_name("config/service"))
      .add_source(config::Environment::with_prefix("APP").separator("_"))
      .build()?;

    let config: super::config::Config = conf.try_deserialize()?;

    Ok(ServiceLocator {
      clusters: ShardedClient::try_new(
        config.system,
        ClustersClient::new).await?,
      operations: ShardedClient::try_new(
        config.longrunning,
        OperationsClient::new).await?,
      workspace_nodes: ShardedClient::try_new(
        config.cluster,
        WorkspaceNodesClient::new).await?,
    })
  }

  pub async fn get_client(&self, svc: &str) -> Result<ShardedClient<WorkspaceNodesSvcClient>, super::Error> {
    match svc {
      "cluster.WorkspaceNodes" => Ok(self.workspace_nodes.clone()),
      _ => Err(super::Error::MissingClient(ERROR_MISSING_SERVICE.to_string())),
    }
  }

  pub async fn get_clusters_client(&self) -> Result<ShardedClient<ClusterSvcClient>, super::Error> {
    Ok(self.clusters.clone())
  }

  pub async fn get_operations_client(&self) -> Result<ShardedClient<OperationsSvcClient>, super::Error> {
    Ok(self.operations.clone())
  }

  pub async fn get_workspace_nodes_client(&self) -> Result<ShardedClient<WorkspaceNodesSvcClient>, super::Error> {
    Ok(self.workspace_nodes.clone())
  }
}