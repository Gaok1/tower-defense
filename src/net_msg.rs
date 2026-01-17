use serde::{Deserialize, Serialize};

use crate::app::{Enemy, Tower, TowerKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub running: bool,
    pub speed: u8,
    pub money: i32,
    pub lives: i32,
    pub wave: i32,
    pub towers: Vec<Tower>,
    pub enemies: Vec<Enemy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum NetCmd {
    TogglePause,
    CycleSpeed,
    Build { x: u16, y: u16, kind: TowerKind },
    Upgrade { x: u16, y: u16 },
    Sell { x: u16, y: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum NetMsg {
    Hello {
        name: String,
    },
    SetMap {
        map_index: usize,
    },
    StartGame {
        map_index: usize,
    },
    Cursor {
        name: String,
        x: u16,
        y: u16,
    },
    Cmd {
        id: u32,
        cmd: NetCmd,
    },
    CmdResult {
        id: u32,
        ok: bool,
        error: Option<String>,
    },
    State {
        state: GameSnapshot,
    },
}
