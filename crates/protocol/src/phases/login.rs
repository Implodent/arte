use crate::model::Gamemode;

use super::*;

#[derive(Debug)]
pub struct LoginStart {
    pub username: String,
    pub uuid: Option<Uuid>,
}

impl AsyncSerialize for LoginStart {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        try {
            writer.serialize(&self.username).await?;
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
                username: read_string_limit(reader, 16).await?,
                uuid: reader.deserialize().await?,
            }
        }
    }
}

impl Packet for LoginStart {
    const STATE: State = State::Login;
    const ID: VarInt<i32> = VarInt(0x00);
}

#[derive(Debug)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
}

#[derive(Debug)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub signature: Option<StringLimit<16>>
}

impl AsyncSerialize for LoginSuccess {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()> {
        try {
            writer.serialize(&self.uuid).await?;
            writer.serialize(&self.username).await?;
            writer.serialize(&VarInt(0)).await?;
        }
    }
}

impl AsyncDeserializeContexful for LoginSuccess {
    type Context = PacketContext;

    async fn read_with_context(reader: &mut impl ReadExt, context: &Self::Context) -> Result<Self> {
        try {
            Self::check_context(context)?;

            Self {
                uuid: reader.deserialize().await?,
                username: read_string_limit(reader, 16).await?,
                // we don't care about properties
            }
        }
    }
}

impl Packet for LoginSuccess {
    const ID: VarInt<i32> = VarInt(0x02);
    const STATE: State = State::Login;
}

pub struct LoginPlay {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub game_mode: Gamemode
}
