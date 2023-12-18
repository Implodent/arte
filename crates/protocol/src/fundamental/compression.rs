use super::*;

use async_compression::futures::{bufread::ZlibDecoder, write::ZlibEncoder};

pub struct Zlib<T>(pub T);

impl<T: AsyncSerialize> AsyncSerialize for Zlib<T> {
    async fn write_to(&self, writer: &mut impl super::WriteExt) -> crate::Result<()> {
        self.0.write_to(&mut ZlibEncoder::new(writer)).await
    }
}

impl<T: AsyncDeserialize> AsyncDeserialize for Zlib<T> {
    async fn read_from(reader: &mut impl super::ReadExt) -> crate::Result<Self> {
        Ok(Self(
            T::read_from(&mut ZlibDecoder::new(BufReader::new(reader))).await?,
        ))
    }
}

impl<T: AsyncDeserializeContexful> AsyncDeserializeContexful for Zlib<T> {
    type Context = T::Context;

    async fn read_with_context(reader: &mut impl ReadExt, context: &Self::Context) -> Result<Self> {
        Ok(Self(
            T::read_with_context(&mut ZlibDecoder::new(BufReader::new(reader)), context).await?,
        ))
    }
}
