pub mod google {
  pub mod rpc {
    tonic::include_proto!("google.rpc");
  }
}

pub mod longrunning {
  tonic::include_proto!("longrunning");
}

pub mod cluster {
  tonic::include_proto!("cluster");
}

pub mod health {
  pub use tonic_health::*;
}
