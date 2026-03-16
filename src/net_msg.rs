use serde::{Deserialize, Serialize};

use crate::app::{Enemy, TargetMode, Tower, TowerKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub running: bool,
    pub speed: u8,
    pub money: i32,
    pub lives: i32,
    pub wave: i32,
    pub pending_wave_start: bool,
    pub prep_ticks: u32,
    pub towers: Vec<Tower>,
    pub enemies: Vec<Enemy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum NetCmd {
    TogglePause,
    CycleSpeed,
    StartWave,
    Build { x: u16, y: u16, kind: TowerKind },
    Upgrade { x: u16, y: u16 },
    Sell { x: u16, y: u16 },
    SetTargetMode { x: u16, y: u16, mode: TargetMode },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum FxEvent {
    Projectile {
        kind: TowerKind,
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
        ttl: u16,
        seed: u32,
        muzzle_seed: u32,
        tracer_seed: Option<u32>,
    },
    TracerLine {
        kind: TowerKind,
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
        seed: u32,
    },
    Impact {
        kind: TowerKind,
        x: u16,
        y: u16,
        seed: u32,
    },
    ArcLightning {
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
        seed: u32,
    },
    TargetFlash {
        x: u16,
        y: u16,
        seed: u32,
    },
    Dust {
        x: u16,
        y: u16,
        seed: u32,
    },
    Shatter {
        x: u16,
        y: u16,
        seed: u32,
    },
    StatusOverlay {
        target: usize,
        ttl: u8,
        seed: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum NetMsg {
    Hello {
        name: String,
    },
    Kick {
        reason: Option<String>,
    },
    EnterMapSelect {
        map_index: usize,
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
        #[serde(default)]
        pending_build: Option<(u16, u16, TowerKind)>,
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
    Fx {
        events: Vec<FxEvent>,
    },
}
