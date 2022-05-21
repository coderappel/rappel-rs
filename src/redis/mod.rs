use redis::FromRedisValue;
use redis::RedisError;
use redis::RedisResult;
use redis::RedisWrite;
use redis::ToRedisArgs;
use redis::Value;
pub use redis::*;

#[derive(Debug, Clone)]
pub struct ProtoValue<T: prost::Message>(pub T);

impl<T: prost::Message> From<T> for ProtoValue<T> {
  fn from(v: T) -> Self {
    ProtoValue(v)
  }
}

impl<T: prost::Message + std::default::Default> FromRedisValue for ProtoValue<T> {
  fn from_redis_value(v: &Value) -> RedisResult<Self> {
    let buf: bytes::Bytes = redis::FromRedisValue::from_redis_value(v)?;

    prost::Message::decode(buf)
      .map(|v| ProtoValue(v))
      .map_err(|e| {
        RedisError::from((
          redis::ErrorKind::IoError,
          "Deserialization failed",
          e.to_string(),
        ))
      })
  }
}

impl<T: prost::Message + std::default::Default> ToRedisArgs for ProtoValue<T> {
  fn write_redis_args<W>(&self, out: &mut W)
  where
    W: ?Sized + RedisWrite,
  {
    let mut buf = bytes::BytesMut::with_capacity(self.0.encoded_len());
    if let Err(error) = self.0.encode(&mut buf) {
      tracing::debug!(message = "Proto encode failed", %error)
    }
    out.write_arg(&buf);
  }
}
