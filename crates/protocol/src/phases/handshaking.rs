use super::*;

#[derive(Debug)]
pub struct Handshake {
    pub protocol_version: VarInt,
    pub address: String,
    pub port: u16,
    pub next_state: State,
}

impl Packet for Handshake {
    const ID: VarInt<i32> = VarInt(0x00);
    const STATE: State = State::Handshaking;
}

impl AsyncDeserializeContexful for Handshake {
    type Context = PacketContext;

    async fn read_with_context(reader: &mut impl ReadExt, context: &Self::Context) -> Result<Self> {
        try {
            Self::check_context(context)?;

            Self {
                protocol_version: reader.deserialize().await?,
                address: string_limit(reader, 256).await?,
                port: reader.deserialize().await?,
                next_state: read_enum! { [reader.deserialize::<VarInt>().await?.0]
                    1 => State::Status,
                    2 => State::Play
                },
            }
        }
    }
}

impl AsyncSerialize for Handshake {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        try {
            writer.serialize(&self.protocol_version).await?;
            writer.serialize(&self.address).await?;
            writer.serialize(&self.port).await?;
            writer
                .serialize(&match self.next_state {
                    State::Status => 1,
                    State::Play => 2,
                    other => {
                        return Err(Error::BadEnumValue {
                            values: vec![
                                String::from("State::Status"),
                                String::from("State::Play"),
                            ],
                            got: format!("{other:?}"),
                        })
                    }
                })
                .await?;
        }
    }
}
