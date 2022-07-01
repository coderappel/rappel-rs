use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::Serialize;

use super::Codec;
use super::Decoder;
use super::DecoderRead;
use super::Encoder;
use super::EncoderWrite;

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error("Serde failed {0}")]
  Serde(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Default)]
pub struct SerdeJsonEncoder<T>(PhantomData<T>);

#[derive(Clone, Debug, Default)]
pub struct SerdeJsonDecoder<T>(PhantomData<T>);

impl<U: Serialize> Encoder for SerdeJsonEncoder<U> {
  type Item = U;

  type Error = Error;

  fn encode<T: EncoderWrite>(
    &mut self,
    item: &Self::Item,
    buf: &mut T,
  ) -> Result<usize, Self::Error> {
    let temp = serde_json::to_vec(item)?;
    buf.write(&temp);

    Ok(temp.len())
  }
}

impl<U: DeserializeOwned> Decoder for SerdeJsonDecoder<U> {
  type Item = U;

  type Error = Error;

  fn decode<T: DecoderRead>(&mut self, buf: &mut T) -> Result<Option<Self::Item>, Self::Error> {
    let res = serde_json::from_slice(buf.as_slice())?;
    Ok(Some(res))
  }
}

#[derive(Clone, Debug)]
pub struct JsonCodec<T, U>(PhantomData<(T, U)>);

impl<T: Serialize, U: DeserializeOwned> JsonCodec<T, U> {
  pub fn new() -> Self {
    Self::default()
  }
}

impl<T: Serialize, U: DeserializeOwned> Default for JsonCodec<T, U> {
  fn default() -> Self {
    Self(PhantomData)
  }
}

impl<T, U> Codec for JsonCodec<T, U>
where
  T: Serialize,
  U: DeserializeOwned,
{
  type Encodable = T;
  type Decodable = U;
  type EncodingError = Error;
  type DecodingError = Error;
  type Encoder = SerdeJsonEncoder<T>;
  type Decoder = SerdeJsonDecoder<U>;

  fn encoder(&self) -> Self::Encoder {
    SerdeJsonEncoder(PhantomData)
  }

  fn decoder(&self) -> Self::Decoder {
    SerdeJsonDecoder(PhantomData)
  }
}
