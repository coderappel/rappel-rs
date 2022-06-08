use crate::proto::cluster::NodePhase;
use crate::proto::cluster::NodeStatus;
use crate::proto::cluster::WorkspaceNode;
use crate::proto::prelude::ProstTimestamp;
use k8s_openapi::api::core::v1::Pod;
use kube::ResourceExt;
use std::collections::HashMap;

pub trait PodExt {
  fn into_workspace_node(&self) -> WorkspaceNode;

  fn is_running(&self) -> bool;

  fn is_pending(&self) -> bool;

  fn is_succeeded(&self) -> bool;

  fn is_failed(&self) -> bool;

  fn is_unknown(&self) -> bool;
}

impl PodExt for Pod {
  fn into_workspace_node(&self) -> WorkspaceNode {
    let mut labels = HashMap::default();
    labels.extend(self.labels().clone());
    let mut annotations = HashMap::default();
    annotations.extend(self.annotations().clone());

    let mut node = WorkspaceNode::default();
    node.node_id = self.name();

    node.cluster_id = labels.remove("rappel_cluster_id").unwrap();
    node.userspace = labels.remove("rappel_userspace").unwrap();
    node.namespace = self.namespace().unwrap();

    node.labels = labels;
    node.annotations = annotations;

    if let Some(spec) = &self.spec {
      if let Some(sa) = &spec.service_account_name {
        node.service_account_name = sa.clone();
      }
    }

    let mut node_status = NodeStatus::default();

    node_status.phase = if self.is_running() {
      NodePhase::Running
    } else if self.is_pending() {
      NodePhase::Pending
    } else if self.is_succeeded() || self.is_failed() {
      NodePhase::Terminated
    } else {
      NodePhase::Unknown
    }
    .into();

    if let Some(status) = &self.status {
      if let Some(ts) = &status.start_time {
        let timestamp = ProstTimestamp::from(ts.0).into_inner();
        node_status.started_at = Some(timestamp);
      }

      if let Some(message) = &status.message {
        node_status.phase_message = message.clone();
      }

      if let Some(reason) = &status.reason {
        node_status.phase_reason = reason.clone();
      }

      if self.is_failed() || self.is_succeeded() {
        if let Some(s) = &status.container_statuses {
          node_status.exit_code = s
            .iter()
            .map(|c| c.state.clone().unwrap().terminated.unwrap().exit_code)
            .max()
            .unwrap()
        }
      }
    }

    node.status = Some(node_status);

    node
  }

  fn is_running(&self) -> bool {
    if let Some(status) = &self.status {
      if let Some(phase) = &status.phase {
        return phase == "Running";
      }
    }
    false
  }

  fn is_pending(&self) -> bool {
    if let Some(status) = &self.status {
      if let Some(phase) = &status.phase {
        return phase == "Pending";
      }
    }
    false
  }

  fn is_succeeded(&self) -> bool {
    if let Some(status) = &self.status {
      if let Some(phase) = &status.phase {
        return phase == "Succeeded";
      }
    }
    false
  }

  fn is_failed(&self) -> bool {
    if let Some(status) = &self.status {
      if let Some(phase) = &status.phase {
        return phase == "Failed";
      }
    }
    false
  }

  fn is_unknown(&self) -> bool {
    if let Some(status) = &self.status {
      if let Some(phase) = &status.phase {
        return phase == "Unknown";
      }
    }
    true
  }
}
