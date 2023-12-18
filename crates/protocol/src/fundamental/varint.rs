use super::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct VarInt<T: VarNumber = i32>(pub T);

impl<T: VarNumber> VarInt<T> {
    pub fn usize(u: usize) -> Self
    where
        T: TryFrom<usize>,
        <T as TryFrom<usize>>::Error: std::fmt::Debug,
    {
        Self(u.try_into_into::<T>().unwrap())
    }
    pub fn to_usize(&self) -> usize
    where
        T: TryInto<usize>,
        T::Error: std::fmt::Debug,
    {
        self.0.try_into().unwrap()
    }
    pub fn length(&self) -> usize {
        self.0.length()
    }
}

impl<T: VarNumber> AsyncSerialize for VarInt<T> {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        self.0.write_var(writer).await
    }
}

impl<T: VarNumber> AsyncDeserialize for VarInt<T> {
    async fn read_from(reader: &mut impl ReadExt) -> Result<Self> {
        T::read_var(reader).await.map(Self)
    }
}

// a trait to abstract away how the numbers are written to the streams
#[doc(hidden)]
pub trait VarNumber: Sized + Copy {
    async fn read_var(reader: &mut impl ReadExt) -> Result<Self>;
    async fn write_var(self, writer: &mut impl WriteExt) -> Result<()>;
    fn length(&self) -> usize;
}

macro_rules! impl_varnum_signed {
    ($($ty:ty),*) => {
        $(impl VarNumber for $ty {
            async fn read_var(reader: &mut impl ReadExt) -> Result<Self> {
                let mut result = 0;
                let mut shift = 0;
                let mut byte: u8;

                loop {
                    byte = reader.byte().await?;
                    result |= Self::from(byte & 0x7f) << shift;
                    shift += 7;

                    if (byte & 0x80) == 0 {
                        break;
                    }
                }

                if (shift < Self::BITS) && ((byte & 0x40) != 0) {
                    result |= !0 << shift;
                }

                Ok(result)
            }

            async fn write_var(mut self, writer: &mut impl WriteExt) -> Result<()> {
                loop {
                    let byte = (self as u8) & 0x7f;
                    self >>= 7;
                    let more = !(((self == 0) && ((byte & 0x40) == 0))
                        || ((self == -1) && ((byte & 0x40) != 0)));

                    writer
                        .byte(byte | more.then_some(0x80).unwrap_or_default())
                        .await?;

                    if !more {
                        break;
                    }
                }

                Ok(())
            }

            fn length(&self) -> usize {
                let mut value = *self;
                let mut len = 0;

                loop {
                    let byte = (value as u8) & 0x7f;
                    value >>= 7;
                    let more = !(((value == 0) && ((byte & 0x40) == 0))
                        || ((value == -1) && ((byte & 0x40) != 0)));

                    len += 1;

                    if !more {
                        break;
                    }
                }

                len
            }
        })*
    };
}

impl_varnum_signed!(i32, i64);
