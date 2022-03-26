use tonic::transport::Channel;

use crate::proto::cluster::workspace_nodes_client::WorkspaceNodesClient;

use super::client::ShardedClient;

#[derive(Debug, Clone)]
pub struct ServiceLocator {
  config: super::config::Config,
  workspace_nodes: ShardedClient<WorkspaceNodesClient<Channel>>
}

impl ServiceLocator {

  pub async fn try_new() -> Result<ServiceLocator, super::Error> {
    let conf = config::Config::builder()
      .add_source(config::File::with_name("config/service"))
      .add_source(config::Environment::with_prefix("APP").separator("_"))
      .build()?;

    let config: super::config::Config = conf.try_deserialize()?;

    Ok(ServiceLocator {
      config: config.clone(),
      workspace_nodes: ShardedClient::try_new(
        config.cluster, 
        WorkspaceNodesClient::new).await?,
    })
  }

  pub async fn workspace_nodes(&self) -> Result<ShardedClient<WorkspaceNodesClient<Channel>>, super::Error> {
    Ok(self.workspace_nodes.clone())
  }

}
