pub mod google {
  pub mod rpc {
    tonic::include_proto!("google.rpc");
  }
}

pub mod app {
  pub mod resource {
    tonic::include_proto!("app.resource");
  }

  pub mod workspace {
    tonic::include_proto!("app.workspace");
  }

  pub mod session {
    tonic::include_proto!("app.session");
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

pub mod rappel {
  pub mod process {
    tonic::include_proto!("rappel.process");
  }

  pub mod workspace {
    tonic::include_proto!("rappel.workspace");
  }
}

pub use rappel::process;
pub use rappel::workspace;
