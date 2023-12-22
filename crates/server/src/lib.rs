#![feature(try_blocks)]

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use arte_protocol::{
    fundamental::SerializedPacket,
    phases::{handshaking::Handshake, login::LoginStart},
    *,
};
use async_std::{
    channel::{Receiver, Sender},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use futures::{select_biased, FutureExt};
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

    let LoginStart {
        username: name,
        uuid,
    } = network.recv_packet().await?;
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

    debug!(network.peer_addr = %network.tcp.peer_addr()?, ?handshake);

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
    pub async fn accept(
        server: &mut Server,
        tcp: TcpStream,
        addr: SocketAddr,
        error_sender: Sender<(SocketAddr, Error)>,
    ) -> Result<Option<Arc<Mutex<Self>>>> {
        try {
            let mut net = PlayerNetwork {
                tcp,
                state: State::Handshaking,
                compressing: false,
                compression_threshold: server.compression,
            };

            match handshake(&mut net).await? {
                State::Play => {
                    let player = Arc::new(Mutex::new(login(net).await?));
                    async_std::task::spawn({
                        let player = player.clone();
                        let compression = server.compression;
                        async move {
                            if let Err(e) = Self::play(player, compression).await {
                                error_sender.send((addr, e)).await.unwrap();
                            }
                        }
                    });
                    Some(player.clone())
                }
                State::Status => {
                    status(&mut net).await?;
                    None
                }
                state => panic!("invalid state {state:?}"),
            }
        }
    }

    async fn play(me: Arc<Mutex<Self>>, compression: Option<usize>) -> Result<()> {
        try {
            let tcp = me.lock_arc().await.network.tcp.clone();
            loop {
                let packet =
                    SerializedPacket::read_packet(&mut &tcp, compression.is_some(), State::Play)
                        .await?;
            }
        }
    }
}

pub struct Server {
    pub players: HashMap<SocketAddr, Arc<Mutex<ServerPlayer>>>,
    error: (Sender<(SocketAddr, Error)>, Receiver<(SocketAddr, Error)>),
    pub compression: Option<usize>,
    pub tcp: TcpListener,
}

impl Server {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            select_biased! {
                stream = self.tcp.accept().fuse() => {
                    let (stream, addr) = stream?;

                    if let Some(player) = ServerPlayer::accept(self, stream, addr, self.error.0.clone()).await? {
                        if self.players.insert(addr, player).is_some() {
                            error!("Disconnecting old player with {addr} because another one joined");
                        }
                    }
                },
                errored = self.error.1.recv().fuse() => {
                    if let Ok((addr, error)) = errored {
                        self.players.remove(&addr);
                        error!(%addr, %error, "Error sent from Play phase");
                    } else {
                        return Ok(());
                    }
                }
            }
        }
    }
}
