extern crate core;

#[cfg(feature = "proto")]
pub mod grpc;

pub mod id;

#[cfg(feature = "proto")]
pub mod proto;

#[cfg(feature = "longrunning")]
pub mod longrunning;

#[cfg(feature = "proto")]
pub mod prost;

#[cfg(feature = "proto")]
pub mod kube;

#[cfg(feature = "redis-tokio")]
pub mod redis;

pub mod service;
