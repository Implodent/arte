use super::*;

#[derive(Debug)]
pub struct LoginStart {
    pub name: String,
    pub uuid: Option<Uuid>,
}

impl AsyncSerialize for LoginStart {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        try {
            writer.serialize(&self.name).await?;
            writer.serialize(&self.uuid).await?;
        }
    }
}

impl AsyncDeserializeContexful for LoginStart {
    type Context = PacketContext;

    async fn read_with_context(reader: &mut impl ReadExt, context: &Self::Context) -> Result<Self> {
        try {
            Self::check_context(context)?;

            Self {
                name: read_string_limit(reader, 16).await?,
                uuid: reader.deserialize().await?,
            }
        }
    }
}

impl Packet for LoginStart {
    const STATE: State = State::Login;
    const ID: VarInt<i32> = VarInt(0x00);
}
