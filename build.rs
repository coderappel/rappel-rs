use std::{env, path::PathBuf};

fn main() {
  let descriptor_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("descriptors.bin");

  tonic_build::configure()
    .build_server(true)
    .compile_well_known_types(true)
    .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
    .file_descriptor_set_path(&descriptor_path)
    .compile(
      &[
        "proto/rappel/system/clusters.proto",
        "proto/rappel/system/location.proto",
        "proto/rappel/cluster/workspaces.proto",
        "proto/rappel/account/billing.proto",
        "proto/rappel/account/organization.proto",
        "proto/rappel/workspace/ides.proto",
        "proto/rappel/workspace/workspaces.proto",
        "proto/rappel/process/processes.proto",
        "proto/google/protobuf/any.proto",
      ],
      &["proto"],
    )
    .unwrap();

  println!("cargo:rerun-if-changed=Cargo.toml");
  println!("cargo:rerun-if-changed=migrations");
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=proto/google/protobuf/any.proto");
  println!("cargo:rerun-if-changed=proto/rappel/cluster/workspaces.proto");
  println!("cargo:rerun-if-changed=proto/rappel/workspace/ides.proto");
  println!("cargo:rerun-if-changed=proto/rappel/workspace/workspaces.proto");

  std::fs::copy(&descriptor_path, "./descriptors.pb").unwrap();
}
