use async_std::io;

use super::*;

pub struct PacketData {
    pub packet_id: VarInt,
    pub data: Bytes,
}

impl AsyncDeserializeContexful for PacketData {
    type Context = u64;

    async fn read_with_context(reader: &mut impl ReadExt, length: &Self::Context) -> Result<Self> {
        try {
            let packet_id: VarInt<i32> = reader.deserialize().await?;
            Self {
                packet_id,
                data: reader
                    .take(*length - packet_id.length() as u64)
                    .collect()
                    .await?,
            }
        }
    }
}

impl AsyncSerialize for PacketData {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        try {
            writer
                .serialize(&VarInt::<i32>(
                    (self.packet_id.length() + self.data.len())
                        .try_into()
                        .unwrap(),
                ))
                .await?;
            writer.serialize(&self.packet_id).await?;
            io::copy(&self.data[..], writer).await?;
        }
    }
}

pub enum SerializedPacket {
    Uncompressed(PacketData),
    Compressed(Zlib<PacketData>),
}

impl SerializedPacket {
    pub async fn read_packet<T: AsyncDeserializeContexful<Context = PacketContext>>(
        reader: &mut impl ReadExt,
        compressing: bool,
        state: State,
    ) -> Result<T> {
        if compressing {
            let packet_length = VarInt::<i32>::read_from(reader).await?;
            let VarInt(data_length) = VarInt::<i32>::read_from(reader).await?;

            let id = VarInt::<i32>::read_from(reader).await?;
            if data_length == 0 {
                let data_length =
                    packet_length.0.try_into_into::<u64>().unwrap() - id.length() as u64;

                T::read_with_context(&mut reader.take(data_length), &PacketContext { id, state })
                    .await
            } else {
                Ok(Zlib::<T>::read_with_context(
                    &mut reader
                        .take(data_length.try_into_into::<u64>().unwrap() - id.length() as u64),
                    &PacketContext { id, state },
                )
                .await?
                .0)
            }
        } else {
            let length = VarInt::<i32>::read_from(reader).await?;
            let id = VarInt::<i32>::read_from(reader).await?;

            let data_length = length.0.try_into_into::<u64>().unwrap() - id.length() as u64;

            T::read_with_context(&mut reader.take(data_length), &PacketContext { id, state }).await
        }
    }

    pub async fn write_to(
        &self,
        writer: &mut impl WriteExt,
        compression_threshold: Option<usize>,
    ) -> Result<()> {
        try {
            match self {
                Self::Uncompressed(data) => data.write_to(writer).await?,
                Self::Compressed(data) => {
                    let data_len = data.0.packet_id.length() + data.0.data.len();
                    let data_length = VarInt::<i32>(data_len.try_into().unwrap());

                    if let Some(threshold) = compression_threshold {
                        if data_len < threshold {
                            writer.serialize(&data_length).await?;
                            writer.serialize(&VarInt(0i32)).await?;

                            return data.0.write_to(writer).await;
                        }
                    }

                    let mut zlib_buf = vec![];

                    data.write_to(&mut zlib_buf).await?;

                    // length of data length + compressed length of (packet id + data)
                    let packet_len = data_length.length() + zlib_buf.len();
                    let packet_length = VarInt::<i32>(packet_len.try_into().unwrap());

                    writer.serialize(&packet_length).await?;
                    writer.serialize(&data_length).await?;
                    io::copy(&zlib_buf[..], writer).await?;
                }
            }
        }
    }

    pub async fn read_from(reader: &mut impl ReadExt, compressing: bool) -> Result<Self> {
        if compressing {
            Self::read_compressed(reader).await
        } else {
            Self::read_normal(reader).await
        }
    }

    async fn read_normal(reader: &mut impl ReadExt) -> Result<Self> {
        try {
            let length = VarInt::<i32>::read_from(reader).await?;

            Self::Uncompressed(
                reader
                    .deserialize_with_context(&length.0.try_into().unwrap())
                    .await?,
            )
        }
    }

    async fn read_compressed(reader: &mut impl ReadExt) -> Result<Self> {
        try {
            let packet_length = VarInt::<i32>::read_from(reader).await?;
            let VarInt(data_length) = VarInt::<i32>::read_from(reader).await?;

            // very funny
            if data_length == 0 {
                Self::Uncompressed(
                    reader
                        .deserialize_with_context(&packet_length.0.try_into().unwrap())
                        .await?,
                )
            } else {
                Self::Compressed(
                    reader
                        .deserialize_with_context(&packet_length.0.try_into().unwrap())
                        .await?,
                )
            }
        }
    }
}
