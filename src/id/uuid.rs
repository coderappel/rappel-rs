use crate::id::UidGenerator;
use core::marker::PhantomData;
use uuid::Uuid;

/// UUID V4 generator
#[derive(Debug, Clone)]
pub struct UuidGenerator {
  _guard: PhantomData<i32>,
}

impl UuidGenerator {
  pub fn new() -> Self {
    UuidGenerator::default()
  }
}

impl Default for UuidGenerator {
  fn default() -> Self {
    Self {
      _guard: PhantomData::default(),
    }
  }
}

impl UidGenerator for UuidGenerator {
  type Item = Uuid;
  type Error = ();

  fn next(&mut self) -> Result<Self::Item, Self::Error> {
    Ok(Uuid::new_v4())
  }
}

#[cfg(test)]
mod tests {
  use crate::id::{UidGenerator, UuidGenerator};

  #[test]
  fn test_generate_uuid() {
    let mut gen = UuidGenerator::new();
    let mut last_id = gen.next().expect("Expected ID");

    let mut count = 1;

    loop {
      count += 1;

      let id = gen.next().expect("Expected ID");
      assert_ne!(id, last_id);
      last_id = id;

      if count == 1 << 13 {
        break;
      }
    }
  }
}
