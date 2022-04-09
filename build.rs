fn main() {
  tonic_build::configure()
    .build_server(true)
    .file_descriptor_set_path("./devbox.pb")
    .compile(
      &[
        "proto/cluster/clusters.proto",
        "proto/cluster/workspace_nodes.proto",
        "proto/app/workspace.proto",
        "proto/app/resource.proto",
        "proto/app/session.proto",
        "proto/rappel/workspace/ides.proto",
        "proto/rappel/workspace/workspaces.proto",
        "proto/rappel/process/processes.proto",
      ],
      &["proto"],
    )
    .unwrap();

  println!("cargo:rerun-if-changed=Cargo.toml");
  println!("cargo:rerun-if-changed=migrations");
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=proto/cluster/workspace_nodes.proto");
  println!("cargo:rerun-if-changed=proto/rappel/workspace/ides.proto");
  println!("cargo:rerun-if-changed=proto/rappel/workspace/workspaces.proto");
}
