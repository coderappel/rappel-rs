mod snowflake;
mod uuid;

pub use super::id::snowflake::*;
pub use super::id::uuid::*;

pub trait UidGenerator {
  type Item;

  type Error;

  fn next(&mut self) -> Result<Self::Item, Self::Error>;
}
