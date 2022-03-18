use std::ops::Deref;
pub use prost_types::Timestamp;

#[derive(Clone, Debug)]
pub struct ProstTimestamp(pub Timestamp);

impl ProstTimestamp {
  pub fn into_inner(self) -> Timestamp {
    self.0
  }
}

impl Deref for ProstTimestamp {
    type Target = Timestamp;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<chrono::DateTime<chrono::Utc>> for ProstTimestamp {
    fn from(timestamp: chrono::DateTime<chrono::Utc>) -> Self {
      let seconds = timestamp.timestamp();
      let nanos = timestamp.timestamp_subsec_nanos() as i32;
      Self(prost_types::Timestamp { nanos, seconds })
    }
}
