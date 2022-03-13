fn main() {
  tonic_build::configure()
    .build_server(true)
    .file_descriptor_set_path("./devbox.pb")
    .compile(
      &[
        "proto/cluster/nodes.proto",
        "proto/app/workspace.proto",
        "proto/app/resource.proto",
      ],
      &["proto"],
    )
    .unwrap();

  println!("cargo:rerun-if-changed=Cargo.toml");
  println!("cargo:rerun-if-changed=migrations");
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=proto/cluster/nodes.proto");
  println!("cargo:rerun-if-changed=proto/longrunning/operations.proto");
}
