use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// Contains the SystemTime on 2022/01/01 00:00:00 UTC
pub const SNOWFLAKE_EPOCH: i64 = 1640998800;

/// The Error type generated by next() on Snowflake generator.
#[derive(thiserror::Error, Debug)]
pub enum SnowflakeError {
  #[error("Clock moved backwards")]
  ClockMovedBackwards,
}

/// Twitter Snowflake inspired id generator. The generator is optimized to cycle timestamp
/// only after it has exhaused the sequence bits.
///
/// ```rust
/// use rappel::id::UidGenerator;
/// use rappel::id::Snowflake;
///
/// let mut gen = Snowflake::new(123);
/// let mut id = gen.next().expect("Expected ID");
/// ```
///
/// The generator produces 64 bit id with the following scheme
/// [0][timestamp millis (41 bits)][machine id (10 bits)][sequence (12 bits)]
///
/// For timestamp, the generator produces referring to 2021/01/01 00:00:00 UTC not
/// UNIX EPOCH.
///
#[derive(Debug, Clone)]
pub struct Snowflake {
  ts_epoch: i64,
  machine_id: i64,
  sequence: i64,
  timestamp_millis: Arc<Mutex<i64>>,
}

impl Snowflake {
  /// Constructs a new instance of Snowflake id generator.
  pub fn new(machine_id: i64) -> Self {
    Self {
      ts_epoch: SNOWFLAKE_EPOCH,
      machine_id: machine_id & 0x03FF,
      sequence: 0,
      timestamp_millis: Arc::new(Mutex::new(Self::curr_time())),
    }
  }

  fn curr_time() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("should read time").as_millis() as i64
  }
}

impl super::UidGenerator for Snowflake {
  type Item = i64;

  type Error = SnowflakeError;

  /// Generates and returns the next id from the generator. The function
  /// is thread safe and blocks when the sequence is exhausted.
  fn next(&mut self) -> Result<Self::Item, Self::Error> {
    let mut last_ts = self.timestamp_millis.lock().unwrap();

    self.sequence = (self.sequence + 1) & 0x0FFF;

    if self.sequence == 0 {
      let mut curr_ts = Snowflake::curr_time();

      if curr_ts < *last_ts {
        return Err(SnowflakeError::ClockMovedBackwards);
      }

      while curr_ts == *last_ts {
        curr_ts = Snowflake::curr_time();
      }

      self.sequence = 1;
      *last_ts = curr_ts;
    }

    let id = ((*last_ts - self.ts_epoch) << 22)
      | (self.machine_id << 12)
      | (self.sequence);

    Ok(id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::id::UidGenerator;

  #[test]
  fn test_create_snowflake() {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("should read time").as_millis() as i64;
    let curr_time = Snowflake::curr_time();

    assert!(curr_time - now >= 0);
  }

  #[test]
  fn test_generate_in_loop() {
    let mut gen = Snowflake::new(123);
    let mut last_id = gen.next().expect("Expected ID");

    let mut count = 1;

    loop {
      count += 1;

      let id = gen.next().expect("Expected ID");
      assert!(id > last_id);
      last_id = id;

      if count == 1 << 13 {
        break;
      }
    }
  }
}
