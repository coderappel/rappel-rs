use tonic::transport::Channel;

use crate::proto::cluster::workspace_nodes_client::WorkspaceNodesClient;

use super::client::ShardedClient;

#[derive(Debug, Clone)]
pub struct ServiceLocator {
  workspace_nodes: ShardedClient<WorkspaceNodesClient<Channel>>
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
      workspace_nodes: ShardedClient::try_new(
        config.cluster, 
        WorkspaceNodesClient::new).await?,
    })
  }

  pub async fn get_client(&self, svc: &str) -> Result<ShardedClient<WorkspaceNodesClient<Channel>>, super::Error> {
    match svc {
      "cluster.WorkspaceNodes" => Ok(self.workspace_nodes.clone()),
      _ => Err(super::Error::MissingClient(ERROR_MISSING_SERVICE.to_string())),
    }
  }

}
