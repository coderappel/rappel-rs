use std::collections::HashMap;
use crate::proto::cluster::Node as ProtoNode;
use crate::proto::cluster::NodeStatus as ProtoNodeStatus;
use crate::prost::ProstTimestamp;
use kube::ResourceExt;
use k8s_openapi::api::core::v1::Pod;

pub trait PodExt {
  fn to_node(&self) -> ProtoNode;

  fn is_running(&self) -> bool;

  fn is_pending(&self) -> bool;

  fn is_succeeded(&self) -> bool;

  fn is_failed(&self) -> bool;

  fn is_unknown(&self) -> bool;
}

impl PodExt for Pod {
  fn to_node(&self) -> ProtoNode {
    let mut labels = HashMap::default();
    labels.extend(self.labels().clone());

    let mut node = ProtoNode::default();
    node.node_id = self.uid().unwrap();
    node.namespace = self.namespace().unwrap();
    node.labels = labels;

    node.node_status = if self.is_running() {
      ProtoNodeStatus::Running
    } else if self.is_pending() {
      ProtoNodeStatus::Starting
    } else if self.is_succeeded() || self.is_failed() {
      ProtoNodeStatus::Terminated
    } else {
      ProtoNodeStatus::Unspecified
    }.into();

    if let Some(status) = &self.status {
      if let Some(ts) = &status.start_time {
        let timestamp = ProstTimestamp::from(ts.0).into_inner();
        node.start_ts = Some(timestamp);
      }
      if self.is_failed() || self.is_succeeded() {
        if let Some(message) = &status.message {
          node.exit_message = message.clone();
        }

        if let Some(s) = &status.container_statuses {
          node.exit_code = s.iter().map(|c| c.state.clone().unwrap().terminated.unwrap().exit_code).max().unwrap()
        }
      }
    }

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
