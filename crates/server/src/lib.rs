use arte_protocol::{phases::handshaking::Handshake, *};
use tracing::*;

pub struct ServerPlayer {
    pub network: PlayerNetwork,
}

impl ServerPlayer {
    async fn status(&mut self) -> Result<()> {
        Ok(())
    }

    async fn login(&mut self) -> Result<()> {
        Ok(())
    }

    async fn handshake(&mut self) -> Result<State> {
        let handshake: Handshake = self.network.recv_packet().await?;
        
        match handshake.next_state {
            State::Status => self.status().await?,
            State::Play => self.login().await?,
            other => error!("invalid handshake state: {other:?}, expected either Status or Play")
        };

        Ok(handshake.next_state)
    }
}
