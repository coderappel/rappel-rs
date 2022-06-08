pub mod json;

pub trait EncoderWrite {
  fn write(&mut self, arg: &[u8]);
}

pub trait DecoderRead {
  fn as_slice(&self) -> &[u8];
}

pub trait Encoder {
  type Item;
  type Error;

  fn encode<T: EncoderWrite>(
    &mut self,
    item: &Self::Item,
    buf: &mut T,
  ) -> Result<usize, Self::Error>;
}

pub trait Decoder {
  type Item;
  type Error;

  fn decode<T: DecoderRead>(&mut self, buf: &mut T) -> Result<Option<Self::Item>, Self::Error>;
}

pub trait Codec {
  type Encodable;
  type EncodingError;
  type Decodable;
  type DecodingError;
  type Encoder: Encoder<Item = Self::Encodable, Error = Self::EncodingError>;
  type Decoder: Decoder<Item = Self::Decodable, Error = Self::DecodingError>;

  fn encoder(&self) -> Self::Encoder;

  fn decoder(&self) -> Self::Decoder;
}

impl<T: bytes::BufMut> EncoderWrite for T {
  fn write(&mut self, arg: &[u8]) {
    self.put_slice(arg)
  }
}

impl DecoderRead for Vec<u8> {
  fn as_slice(&self) -> &[u8] {
    self
  }
}

impl DecoderRead for &str {
  fn as_slice(&self) -> &[u8] {
    self.as_bytes()
  }
}
