use crate::proto::workspace::Workspace;

pub fn generate_for_workspace(w: &Workspace) -> String {
  format!("workspace/{}/operations/{}", w.workspace_id, uuid::Uuid::new_v4().to_string())
}