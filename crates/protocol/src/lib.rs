#![feature(try_blocks)]
#![allow(async_fn_in_trait)]

use bytes::*;

use async_std::{
    io::{BufReader, Read, ReadExt as AsyncStdReadExt, Write},
    net::TcpStream,
};

pub const PROTOCOL_VERSION: VarInt = VarInt(763);

pub mod fundamental;
use fundamental::*;

pub mod model;
pub mod phases;

pub use uuid::{uuid as comptime_uuid, Uuid};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Invalid id/state, expected id {expected_id:?} and state {expected_state:?}, got id {id:?} and state {state:?}")]
    InvalidIdState {
        id: VarInt,
        state: State,
        expected_id: VarInt,
        expected_state: State,
    },
    #[error(
        "String too large, expected a string of max length {limit}, got one with length {length}"
    )]
    StringTooLarge { length: usize, limit: usize },
    #[error("bad enum value, expected any of {values:?}, got {got}")]
    BadEnumValue {
        // for Debug output
        values: Vec<String>,
        // for Debug output
        got: String,
    },
    #[error("invalid protocol version, expected {PROTOCOL_VERSION:?}, got {_0:?}")]
    InvalidProtocolVersion(VarInt),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub trait AsyncDeserialize: Sized {
    async fn read_from(reader: &mut impl ReadExt) -> Result<Self>;
}

pub trait AsyncDeserializeContexful: Sized {
    type Context;

    async fn read_with_context(reader: &mut impl ReadExt, context: &Self::Context) -> Result<Self>;
}

pub trait AsyncSerialize {
    async fn write_to(&self, writer: &mut impl WriteExt) -> Result<()>;
}

pub trait Packet {
    const ID: VarInt<i32>;
    const STATE: State;

    fn check_context(context: &PacketContext) -> Result<()> {
        if context.id != Self::ID || context.state != Self::STATE {
            Err(Error::InvalidIdState {
                id: context.id,
                state: context.state,
                expected_id: Self::ID,
                expected_state: Self::STATE,
            })
        } else {
            Ok(())
        }
    }
}

pub trait PacketHandle<Context>: Packet {
    async fn handle(&self, context: &mut Context) -> Result<()>;
}

pub struct PlayerNetwork {
    pub tcp: TcpStream,
    pub compressing: bool,
    pub state: State,
    pub compression_threshold: Option<usize>,
}

pub struct PacketContext {
    pub id: VarInt,
    pub state: State,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Handshaking,
    Status,
    Login,
    Play,
}

impl PlayerNetwork {
    pub async fn recv_packet<T: AsyncDeserializeContexful<Context = PacketContext>>(
        &mut self,
    ) -> Result<T> {
        SerializedPacket::read_packet(&mut self.tcp, self.compressing, self.state).await
    }

    pub async fn send_packet<T: AsyncSerialize + Packet>(&mut self, packet: T) -> Result<()> {
        let mut data = vec![];

        packet.write_to(&mut data).await?;

        SerializedPacket::Uncompressed(PacketData {
            packet_id: T::ID,
            data: data.into(),
        })
        .write_to(&mut self.tcp, self.compression_threshold)
        .await
    }
}

pub trait Ascribe: Sized {
    fn ascribe<U: AscribeTo<Target = Self>>(self) -> U::Target;
    fn try_into_into<U: TryFrom<Self>>(self) -> Result<U, U::Error>;
}

impl<T> Ascribe for T {
    fn ascribe<U: AscribeTo<Target = Self>>(self) -> U::Target {
        self
    }
    fn try_into_into<U: TryFrom<Self>>(self) -> Result<U, U::Error> {
        self.try_into()
    }
}

#[doc(hidden)]
pub trait AscribeTo {
    type Target;
}

impl<T> AscribeTo for T {
    type Target = T;
}
