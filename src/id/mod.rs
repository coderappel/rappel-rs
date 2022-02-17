pub mod snowflake;

pub trait UidGenerator {
  type Item;

  type Error;

  fn next(&mut self) -> Result<Self::Item, Self::Error>;
}
