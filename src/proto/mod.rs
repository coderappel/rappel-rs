pub use self::rappel::account;
pub use self::rappel::cluster;
pub use self::rappel::process;
pub use self::rappel::rpc;
pub use self::rappel::system;
pub use self::rappel::workspace;

pub mod prelude;

pub mod google {
  pub mod protobuf {
    tonic::include_proto!("google.protobuf");
  }

  pub mod rpc {
    tonic::include_proto!("google.rpc");
  }
}

pub mod longrunning {
  tonic::include_proto!("longrunning");
}

pub mod health {
  pub use tonic_health::*;
}

pub mod rappel {
  pub mod account {
    tonic::include_proto!("rappel.account");
  }

  pub mod cluster {
    tonic::include_proto!("rappel.cluster");
  }

  pub mod process {
    tonic::include_proto!("rappel.process");
  }

  pub mod rpc {
    tonic::include_proto!("rappel.rpc");
  }

  pub mod system {
    tonic::include_proto!("rappel.system");
  }

  pub mod workspace {
    tonic::include_proto!("rappel.workspace");
  }
}

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("descriptors");
