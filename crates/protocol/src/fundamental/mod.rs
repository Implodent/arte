use std::{io::ErrorKind, pin::Pin};

use super::*;

mod compression;
mod packets;
mod varint;

use async_std::io::WriteExt as _;
pub use compression::*;
pub use packets::*;
pub use varint::*;

pub trait ReadExt: async_std::io::ReadExt + Read + Unpin {
    async fn byte(&mut self) -> std::io::Result<u8> {
        let mut byte = 0;

        if let 0 = std::future::poll_fn(|cx| {
            Pin::new(&mut *self).poll_read(cx, std::slice::from_mut(&mut byte))
        })
        .await?
        {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "Unexpected end of file",
            ));
        }

        Ok(byte)
    }

    async fn deserialize<T: AsyncDeserialize>(&mut self) -> Result<T>
    where
        Self: Sized,
    {
        T::read_from(self).await
    }

    async fn deserialize_with_context<T: AsyncDeserializeContexful>(
        &mut self,
        context: &T::Context,
    ) -> Result<T>
    where
        Self: Sized,
    {
        T::read_with_context(self, context).await
    }

    async fn collect(&mut self) -> std::io::Result<Bytes>
    where
        Self: Unpin,
    {
        let mut buf = vec![];

        self.read_to_end(&mut buf).await?;

        Ok(Bytes::from(buf))
    }
}

impl<T: async_std::io::ReadExt + Read + Unpin> ReadExt for T {}

pub trait WriteExt: Write + Unpin {
    async fn serialize<T: AsyncSerialize>(&mut self, thing: &T) -> Result<()>
    where
        Self: Sized + Unpin,
    {
        thing.write_to(self).await
    }
    async fn byte(&mut self, byte: u8) -> std::io::Result<()> {
        if let 0 = std::future::poll_fn(|cx| {
            Pin::new(&mut *self).poll_write(cx, std::slice::from_ref(&byte))
        })
        .await?
        {
            Err(std::io::Error::new(
                ErrorKind::WriteZero,
                "poll_write returned Ok(0)",
            ))
        } else {
            Ok(())
        }
    }
}

impl<T: Write + Unpin> WriteExt for T {}

pub async fn read_string_limit(reader: &mut impl ReadExt, limit: usize) -> Result<String> {
    let length = VarInt::<i32>::read_from(reader)
        .await?
        .0
        .try_into_into::<usize>()
        .unwrap();

    if length > limit {
        return Err(Error::StringTooLarge { length, limit });
    }

    let mut buf = String::new();
    reader.take(length as u64).read_to_string(&mut buf).await?;

    Ok(buf)
}

macro_rules! impl_int_rw {
    ($($int:ty),*$(,)?) => {
        $(
        impl AsyncSerialize for $int {
            async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
                Ok(writer.write_all(&self.to_be_bytes()).await?)
            }
        }

        impl AsyncDeserialize for $int {
            async fn read_from(reader: &mut impl ReadExt) -> Result<Self> {
                let mut buf = [0; Self::BITS as usize / 8];

                reader.read_exact(&mut buf).await?;

                Ok(Self::from_be_bytes(buf))
            }
        }
        )*
    };
}

impl_int_rw![u8, i8, u16, i16, u32, i32, u64, i64, usize, isize];

#[macro_export]
macro_rules! read_enum {
    ([$exp:expr] $($arm:pat$(if $guard:expr)? => $ret_exp:expr),* $(,)?) => {
        match $exp {
            $($arm$(if $guard)? => $ret_exp,)*
            other => return Err(Error::BadEnumValue { values: vec![$(concat!(stringify!($arm), $("if ", stringify!($guard))?).to_owned(),)*], got: format!("{other:?}") })
        }
    };
}

impl AsyncSerialize for String {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        try {
            VarInt::<i32>::usize(self.len()).write_to(writer).await?;
            writer.write_all(self.as_bytes()).await?;
        }
    }
}

impl AsyncDeserialize for String {
    async fn read_from(reader: &mut impl ReadExt) -> Result<Self> {
        try {
            let mut buf = Self::new();
            let length = reader.deserialize::<VarInt>().await?.to_usize();
            reader.take(length as u64).read_to_string(&mut buf).await?;
            buf
        }
    }
}

impl AsyncSerialize for bool {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        Ok(writer.byte(*self as _).await?)
    }
}

impl AsyncDeserialize for bool {
    async fn read_from(reader: &mut impl ReadExt) -> Result<Self> {
        Ok(read_enum!([reader.byte().await?] 0x00 => false, 0x01 => true))
    }
}

impl<T: AsyncSerialize> AsyncSerialize for Option<T> {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        try {
            self.is_some().write_to(writer).await?;
            if let Some(value) = self {
                value.write_to(writer).await?;
            }
        }
    }
}

impl<T: AsyncDeserialize> AsyncDeserialize for Option<T> {
    async fn read_from(reader: &mut impl ReadExt) -> Result<Self> {
        try {
            if reader.deserialize::<bool>().await? {
                Some(reader.deserialize().await?)
            } else {
                None
            }
        }
    }
}

impl<T: AsyncDeserializeContexful> AsyncDeserializeContexful for Option<T> {
    type Context = T::Context;

    async fn read_with_context(reader: &mut impl ReadExt, context: &Self::Context) -> Result<Self> {
        try {
            if reader.deserialize::<bool>().await? {
                Some(reader.deserialize_with_context(context).await?)
            } else {
                None
            }
        }
    }
}

impl AsyncSerialize for Uuid {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        Ok(writer.write_all(self.as_bytes().as_slice()).await?)
    }
}

impl AsyncDeserialize for Uuid {
    async fn read_from(reader: &mut impl ReadExt) -> Result<Self> {
        let mut buf = [0; 16];
        reader.read_exact(&mut buf).await?;

        Ok(Self::from_bytes(buf))
    }
}
