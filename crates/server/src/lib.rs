#![feature(try_blocks)]

use std::sync::Arc;

use arte_protocol::{
    phases::{handshaking::Handshake, login::LoginStart},
    *,
};
use async_std::sync::RwLock;
use tracing::*;

pub struct ServerPlayer {
    pub network: PlayerNetwork,
    pub name: String,
    pub uuid: Uuid,
}

async fn status(network: &mut PlayerNetwork) -> Result<()> {
    Ok(())
}

async fn login(mut network: PlayerNetwork) -> Result<ServerPlayer> {
    network.state = State::Login;

    let LoginStart { username: name, uuid } = network.recv_packet().await?;
    let uuid = uuid.unwrap_or_else(|| {
        debug!(?name, "Player is in offline mode");
        let real = format!("OfflinePlayer:{name}");
        Uuid::new_v3(&Uuid::NAMESPACE_DNS, real.as_bytes())
    });

    Ok(ServerPlayer {
        network,
        name,
        uuid,
    })
}

async fn handshake(network: &mut PlayerNetwork) -> Result<State> {
    let handshake: Handshake = network.recv_packet().await?;

    if handshake.protocol_version != PROTOCOL_VERSION {
        error!(
            "Player's protocol version was not {PROTOCOL_VERSION:?}, it was {:?}",
            handshake.protocol_version
        );

        return Err(Error::InvalidProtocolVersion(handshake.protocol_version));
    }

    Ok(handshake.next_state)
}

impl ServerPlayer {
    pub async fn play(me: Arc<RwLock<Self>>) -> Result<()> {
        try {
            loop {
                let mut this = me.write().await;
            }
        }
    }
}
