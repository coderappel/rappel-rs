mod client;
mod config;
mod error;
mod locator;

pub use client::ShardedClient;
pub use error::Error;
pub use locator::ServiceLocator;
use tonic::transport::Channel;

use crate::proto::cluster::workspace_nodes_client::WorkspaceNodesClient;

pub type WorkspaceNodesShardedClient = ShardedClient<WorkspaceNodesClient<Channel>>;
