#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Gamemode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}
