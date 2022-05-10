fn main() {
  tonic_build::configure()
    .build_server(true)
    .file_descriptor_set_path("./devbox.pb")
    .compile(
      &[
        "proto/rappel/system/clusters.proto",
        "proto/rappel/system/location.proto",
        "proto/rappel/cluster/workspace_nodes.proto",
        "proto/rappel/account/billing.proto",
        "proto/rappel/account/organization.proto",
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
