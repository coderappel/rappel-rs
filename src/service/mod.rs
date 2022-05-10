mod client;
mod config;
mod error;
mod locator;

pub use client::ShardedClient;
pub use error::Error;
pub use locator::ServiceLocator;
use tonic::transport::Channel;

use crate::proto::cluster::workspace_nodes_client::WorkspaceNodesClient;
use crate::proto::longrunning::operations_client::OperationsClient;
use crate::proto::system::clusters_client::ClustersClient;

pub type ClusterSvcClient = ClustersClient<Channel>;
pub type OperationsSvcClient = OperationsClient<Channel>;
pub type WorkspaceNodesSvcClient = WorkspaceNodesClient<Channel>;
pub type WorkspaceNodesShardedClient = ShardedClient<WorkspaceNodesSvcClient>;
