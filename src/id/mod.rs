mod snowflake;

pub use snowflake::*;

pub trait UidGenerator {
  type Item;

  type Error;

  fn next(&mut self) -> Result<Self::Item, Self::Error>;
}
