// Assets Unicode (sprites "desenhados" em grid, com LOD por zoom)
//
// Regras:
// - Só glifos single-width (nada de emoji double-width).
// - Espaço = transparência.
// - Tiles agora são quadrados: tile_w = tile_h = 4*zoom
//
// Objetivo: sprites mais ricos (4x4+), com leitura rápida no combate.

use crate::app::{EnemyKind, TowerKind};

#[derive(Clone, Copy)]
pub struct Sprite {
    pub w: u16,
    pub h: u16,
    pub rows: &'static [&'static str],
}

impl Sprite {
    #[inline]
    pub fn row(&self, y: usize) -> &'static str {
        self.rows.get(y).copied().unwrap_or("")
    }
}

// ---------------------------
// Sprites: Torres (0..=4)
// zoom 0: tile_w = tile_h = 2 (mini)
// zoom 1..=4: tile_w = tile_h = 4*zoom
// ---------------------------

// BASIC (torre padrão / turret)
pub const TOWER_BASIC_Z0: Sprite = Sprite {
    w: 2,
    h: 2,
    rows: &["▣▣", "▄▄"],
};

pub const TOWER_BASIC_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &[" ▄▄ ", "▐██▌", "▐▄▄▌", " ▀▀ "],
};
pub const TOWER_BASIC_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "  ▄▄▄▄  ",
        " ██████▌",
        "▐██████▌",
        "▐██▄▄██▌",
        "▐██████▌",
        " ▀▀▀▀▀▀ ",
        "  ▄▐▌▄  ",
        "  ▀▀▀▀  ",
    ],
};
pub const TOWER_BASIC_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "    ▄▄▄▄    ",
        "   ▄████▄   ",
        "  ▄██▀▀██▄  ",
        " ▄██▌  ▐██▄ ",
        " ██▌ ▄▄ ▐██ ",
        " ██  ███  ██",
        " ██  ███  ██",
        " ██▌ ▀▀ ▐██ ",
        " ▀██▄  ▄██▀ ",
        "  ▀██████▀  ",
        "   ▀████▀   ",
        "    ▀▀▀▀    ",
    ],
};
pub const TOWER_BASIC_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "      ▄▄▄▄      ",
        "     ▄████▄     ",
        "    ▄██▀▀██▄    ",
        "   ▄██▌  ▐██▄   ",
        "  ▄██▌ ▄▄ ▐██▄  ",
        "  ██  ████  ██  ",
        "  ██  ████  ██  ",
        "  ██  ████  ██  ",
        "  ██  ████  ██  ",
        "  ▀█▄ ▀▀▀▀ ▄█▀  ",
        "   ▀██▄  ▄██▀   ",
        "    ▀██████▀    ",
        "     ▀████▀     ",
        "      ▀██▀      ",
        "       ▀▀       ",
        "                ",
    ],
};

// SNIPER (rifle/torre de longo alcance)
pub const TOWER_SNIPER_Z0: Sprite = Sprite {
    w: 2,
    h: 2,
    rows: &["══", "▐▌"],
};

pub const TOWER_SNIPER_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &["══╗ ", "──╢ ", " ▐▌ ", " ▀▀ "],
};
pub const TOWER_SNIPER_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "╞══════╗",
        "└┬─────╝",
        " │ ▄▄   ",
        " │███▄  ",
        " │▀▀▀█  ",
        " │  ▐▌  ",
        " ▀▄▄██▄ ",
        "   ▀▀▀  ",
    ],
};
pub const TOWER_SNIPER_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "╞══════════╗",
        "└┬─────────╝",
        " │  ▄▄▄▄     ",
        " │ ▄████▄    ",
        " │▄██▀▀██▄   ",
        " │██▌  ▐██   ",
        " │██▌  ▐██   ",
        " │▀██▄▄██▀   ",
        " │  ▀██▀     ",
        " │   ▐▌      ",
        " ▀▄▄▄██▄▄▄   ",
        "     ▀▀▀     ",
    ],
};
pub const TOWER_SNIPER_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "╞══════════════╗",
        "└┬─────────────╝",
        " │    ▄▄▄▄       ",
        " │   ▄████▄      ",
        " │  ▄██▀▀██▄     ",
        " │ ▄██▌  ▐██▄    ",
        " │ ██▌ ▄▄ ▐██    ",
        " │ ██  ███  ██   ",
        " │ ██  ███  ██   ",
        " │ ▀██▄  ▄██▀    ",
        " │  ▀██████▀     ",
        " │    ▀██▀       ",
        " │     ▐▌        ",
        " ▀▄▄▄▄██▄▄▄▄     ",
        "      ▀▀▀        ",
        "                 ",
    ],
};

// RAPID (metralhadora / multi-canos)
pub const TOWER_RAPID_Z0: Sprite = Sprite {
    w: 2,
    h: 2,
    rows: &["╦╦", "▀▀"],
};

pub const TOWER_RAPID_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &["╔╦╦╗", "╚╩╩╝", " ▐▌ ", " ▀▀ "],
};
pub const TOWER_RAPID_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "╔══════╗",
        "╠╦╦╦╦╦╣",
        "╠╩╩╩╩╩╣",
        "╚══════╝",
        "  ▄▄▄▄  ",
        " ▄████▄ ",
        " ▀████▀ ",
        "  ▀▀▀▀  ",
    ],
};
pub const TOWER_RAPID_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "╔══════════╗",
        "║╦╦╦╦╦╦╦╦ ║",
        "║╩╩╩╩╩╩╩╩ ║",
        "╚══════════╝",
        "    ▄▄▄▄    ",
        "   ▄████▄   ",
        "  ▄██▀▀██▄  ",
        "  ██▌▐██▌██ ",
        "  ▀██▄▄██▀  ",
        "    ▀██▀    ",
        "     ▐▌     ",
        "    ▀▀▀▀    ",
    ],
};
pub const TOWER_RAPID_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "╔══════════════╗",
        "║╦╦╦╦╦╦╦╦╦╦╦╦╦ ║",
        "║╩╩╩╩╩╩╩╩╩╩╩╩╩ ║",
        "╚══════════════╝",
        "      ▄▄▄▄      ",
        "     ▄████▄     ",
        "    ▄██▀▀██▄    ",
        "   ▄██▌▐██▌██▄   ",
        "   ██▌▐██▌▐███   ",
        "   ▀██▄▄██▄▄██▀  ",
        "     ▀██████▀    ",
        "       ▀██▀      ",
        "        ▐▌       ",
        "      ▀▀▀▀▀▀     ",
        "                 ",
        "                 ",
    ],
};

// CANNON (canhão pesado)
pub const TOWER_CANNON_Z0: Sprite = Sprite {
    w: 2,
    h: 2,
    rows: &["◉◉", "██"],
};

pub const TOWER_CANNON_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &["◉  ◉", "▐██▌", "▐██▌", " ▀▀ "],
};
pub const TOWER_CANNON_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "◉══════◉",
        "  ▄▄▄▄  ",
        " ▄████▄ ",
        "▐██▌▐██▌",
        " ▀██▄▄▀ ",
        "  ▄██▄  ",
        " ◉▐██▌◉ ",
        "  ▀▀▀▀  ",
    ],
};
pub const TOWER_CANNON_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "◉══════════◉",
        "   ▄▄▄▄▄    ",
        "  ▄██████▄  ",
        " ▄██▀▀▀██▄  ",
        "▐██▌   ▐██▌ ",
        "▐██▌   ▐██▌ ",
        " ▀██▄▄▄██▀  ",
        "   ▄████▄   ",
        "  ◉▐████▌◉  ",
        "    ▀██▀    ",
        "     ▐▌     ",
        "    ▀▀▀▀    ",
    ],
};
pub const TOWER_CANNON_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "◉══════════════◉",
        "     ▄▄▄▄▄      ",
        "    ▄██████▄    ",
        "   ▄██▀▀▀██▄    ",
        "  ▄██▌   ▐██▄   ",
        "  ██▌     ▐██   ",
        "  ██▌     ▐██   ",
        "  ▀██▄   ▄██▀   ",
        "   ▀██████▀     ",
        "     ▄████▄     ",
        "    ◉▐████▌◉    ",
        "      ▀██▀      ",
        "       ▐▌       ",
        "     ▀▀▀▀▀▀     ",
        "                ",
        "                ",
    ],
};

// TESLA (bobina elétrica)
pub const TOWER_TESLA_Z0: Sprite = Sprite {
    w: 2,
    h: 2,
    rows: &["╫╫", "▄▄"],
};

pub const TOWER_TESLA_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &[" ▲▲ ", "╫╬╫ ", " ║║ ", " ▀▀ "],
};
pub const TOWER_TESLA_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "  ▲▲▲   ",
        "  ╔╬╗   ",
        " ═╬╬╬═  ",
        "  ╚╬╝   ",
        "  ╫╬╫   ",
        " ▄████▄ ",
        " ██████ ",
        "  ▀▀▀▀  ",
    ],
};
pub const TOWER_TESLA_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "   ║║║║║║   ",
        "   ║╬╬╬╬║   ",
        "  x╬╬╬╬x  ",
        "   ║╬╬╬╬║   ",
        "   ║╬╬╬╬║   ",
        "   ║║║║║║   ",
        "   ▄████▄   ",
        "  ▄██████▄  ",
        "  ████████  ",
        "  ▀██████▀  ",
        "    ▀██▀    ",
        "     ▀▀     ",
    ],
};
pub const TOWER_TESLA_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "    ║║║║║║║║    ",
        "    ║╬╬╬╬╬╬║    ",
        "   x╬╬╬╬╬╬x   ",
        "    ║╬╬╬╬╬╬║    ",
        "    ║╬╬╬╬╬╬║    ",
        "    ║╬╬╬╬╬╬║    ",
        "    ║║║║║║║║    ",
        "     ▄████▄     ",
        "    ▄██████▄    ",
        "   ▄████████▄   ",
        "   ██████████   ",
        "   ▀████████▀   ",
        "     ▀████▀     ",
        "      ▀██▀      ",
        "       ▀▀       ",
        "                ",
    ],
};

// FROST (cristal / gelo)
pub const TOWER_FROST_Z0: Sprite = Sprite {
    w: 2,
    h: 2,
    rows: &["◆◆", "◆◆"],
};

pub const TOWER_FROST_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &[" ▲▲ ", "◆  ◆", " ▼▼ ", "    "],
};
pub const TOWER_FROST_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "    ▲▲   ",
        "  ╱   ╲  ",
        " ╱ ◆◆ ╲ ",
        "╱  ◆◆  ╲",
        "╲  ◆◆  ╱",
        " ╲ ◆◆ ╱ ",
        "  ╲  ╱  ",
        "   ▼▼   ",
    ],
};
pub const TOWER_FROST_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "     *      ",
        "    ╱╳╲     ",
        "   ╱╳╳╲    ",
        "  ╱╳╳╳╲   ",
        " ╱╳╳╳╳╲  ",
        "╱╳╳╳*╳╳╲ ",
        "╲╳╳╳╳╳╳╱ ",
        " ╲╳╳╳╳╱  ",
        "  ╲╳╳╱   ",
        "   ╲╳╱    ",
        "    *     ",
        "           ",
    ],
};
pub const TOWER_FROST_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "       *        ",
        "      ╱╳╲       ",
        "     ╱╳╳╲      ",
        "    ╱╳╳╳╲     ",
        "   ╱╳╳╳╳╲    ",
        "  ╱╳╳╳╳╳╲   ",
        " ╱╳╳╳*╳╳╳╲  ",
        "╱╳╳╳╳╳╳╳╳╳╲ ",
        "╲╳╳╳╳╳╳╳╳╳╱ ",
        " ╲╳╳╳╳╳╳╳╱  ",
        "  ╲╳╳╳╳╳╱   ",
        "   ╲╳╳╳╱    ",
        "    ╲╳╳╱     ",
        "     ╲╳╱      ",
        "      *       ",
        "               ",
    ],
};

// ---------------------------
// Sprites: inimigo / impactos
// ---------------------------

pub const ENEMY_Z0: Sprite = Sprite {
    w: 2,
    h: 2,
    rows: &["○○", "▄▄"],
};

pub const ENEMY_RUNNER_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &[" ►► ", "─██─", " ░░ ", " ▄▄ "],
};
pub const ENEMY_TANK_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &["████", "█▣ █", "████", " ▀▀ "],
};
pub const ENEMY_SWARM_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &["· · ", " ·· ", "· · ", "    "],
};
pub const ENEMY_HEALER_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &[" ▄▄ ", "█✚█", " ██ ", " ▀▀ "],
};
pub const ENEMY_SHIELDED_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &["▐██▌", "█▣ █", "▐██▌", " ▀▀ "],
};
pub const ENEMY_SNEAK_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &[" ◆◆ ", "◆░░◆", " ◆◆ ", "    "],
};
// --- Z2 enemy sprites (8x8) ---
pub const ENEMY_RUNNER_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "   ►►   ",
        "  ─██─  ",
        " ──██── ",
        "████████",
        " ──██── ",
        "  ─██─  ",
        "   ░░   ",
        "   ▄▄   ",
    ],
};
pub const ENEMY_TANK_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "████████",
        "█▄▄▄▄▄▄█",
        "█▌    ▐█",
        "█▌ ▣▣ ▐█",
        "█▌    ▐█",
        "█▄▄▄▄▄▄█",
        "████████",
        " ▀▀▀▀▀▀ ",
    ],
};
pub const ENEMY_SWARM_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "· · · · ",
        " · · ·  ",
        "· ·   · ",
        " ·  · · ",
        "  · · · ",
        " · ·  · ",
        "· · · · ",
        "        ",
    ],
};
pub const ENEMY_SHIELDED_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "  ▄▄▄▄  ",
        "▐▐████▌▌",
        "▐▐██▣█▌▌",
        "▐▐████▌▌",
        "▐▐████▌▌",
        "▐▐████▌▌",
        " ▀████▀ ",
        "  ▀▀▀▀  ",
    ],
};
pub const ENEMY_HEALER_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "  ▄▄▄▄  ",
        " ▄████▄ ",
        "▐█✚██✚█▌",
        "████████",
        "████████",
        "▀██████▀",
        " ▀████▀ ",
        "  ▀▀▀▀  ",
    ],
};
pub const ENEMY_SNEAK_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "   ◆◆   ",
        "  ◆░░◆  ",
        " ◆░░░░◆ ",
        "◆░░░░░░◆",
        "◆░░░░░░◆",
        " ◆░░░░◆ ",
        "  ◆░░◆  ",
        "   ◆◆   ",
    ],
};

// --- Z3 enemy sprites (12x12) ---
pub const ENEMY_RUNNER_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "    ►►      ",
        "   ─██─     ",
        "  ──██──    ",
        " ───██───   ",
        "████████████",
        " ───██───   ",
        "  ──██──    ",
        "   ─██─     ",
        "    ░░      ",
        "    ▄▄      ",
        "            ",
        "            ",
    ],
};
pub const ENEMY_TANK_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "████████████",
        "█▄▄▄▄▄▄▄▄▄▄█",
        "█▌        ▐█",
        "█▌  ▣▣▣▣  ▐█",
        "█▌        ▐█",
        "█▌  ████  ▐█",
        "█▌  ████  ▐█",
        "█▌        ▐█",
        "█▄▄▄▄▄▄▄▄▄▄█",
        "████████████",
        "  ▀▀▀▀▀▀▀▀  ",
        "            ",
    ],
};
pub const ENEMY_SWARM_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "· · · · · · ",
        " · · · · ·  ",
        "· · ·   · · ",
        " ·   · · ·  ",
        "· · · · ·   ",
        "  · · · · · ",
        " · ·   · ·  ",
        "· · · · · · ",
        " · · · · ·  ",
        "· ·   · · · ",
        " · · · · ·  ",
        "            ",
    ],
};
pub const ENEMY_SHIELDED_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "    ▄▄▄▄    ",
        "  ▐▐████▌▌  ",
        " ▐▐██████▌▌ ",
        "▐▐████▣███▌▌",
        "▐▐█████████▌",
        "▐▐█████████▌",
        "▐▐█████████▌",
        " ▐▐███████▌▌",
        "  ▐▐█████▌▌ ",
        "   ▀██████▀ ",
        "    ▀████▀  ",
        "     ▀▀▀▀   ",
    ],
};
pub const ENEMY_HEALER_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "    ▄▄▄▄    ",
        "   ▄████▄   ",
        "  ▄██████▄  ",
        " ▐█✚████✚█▌ ",
        "▐███████████",
        "████████████",
        "████████████",
        " ▀█████████▀",
        "  ▀████████▀",
        "   ▀██████▀ ",
        "    ▀████▀  ",
        "     ▀▀▀▀   ",
    ],
};
pub const ENEMY_SNEAK_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "     ◆◆     ",
        "    ◆░░◆    ",
        "   ◆░░░░◆   ",
        "  ◆░░░░░░◆  ",
        " ◆░░░░░░░░◆ ",
        "◆░░░░░░░░░░◆",
        "◆░░░░░░░░░░◆",
        " ◆░░░░░░░░◆ ",
        "  ◆░░░░░░◆  ",
        "   ◆░░░░◆   ",
        "    ◆░░◆    ",
        "     ◆◆     ",
    ],
};

// --- Z4 enemy sprites (16x16) ---
pub const ENEMY_RUNNER_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "      ►►        ",
        "     ─██─       ",
        "    ──██──      ",
        "   ───██───     ",
        "  ────██────    ",
        "████████████████",
        "  ────██────    ",
        "   ───██───     ",
        "    ──██──      ",
        "     ─██─       ",
        "      ░░        ",
        "      ▄▄        ",
        "                ",
        "                ",
        "                ",
        "                ",
    ],
};
pub const ENEMY_TANK_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "████████████████",
        "█▄▄▄▄▄▄▄▄▄▄▄▄▄▄█",
        "█▌            ▐█",
        "█▌  ▣▣▣▣▣▣▣▣  ▐█",
        "█▌            ▐█",
        "█▌  ██████████▌▐█",
        "█▌  ██████████▌▐█",
        "█▌  ██████████▌▐█",
        "█▌  ██████████▌▐█",
        "█▌            ▐█",
        "█▄▄▄▄▄▄▄▄▄▄▄▄▄▄█",
        "████████████████",
        "   ▀▀▀▀▀▀▀▀▀▀   ",
        "                ",
        "                ",
        "                ",
    ],
};
pub const ENEMY_SWARM_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "· · · · · · · · ",
        " · · · · · · ·  ",
        "· · ·   · · · · ",
        " ·   · · · · ·  ",
        "· · · · ·   · · ",
        "  · · · · · ·   ",
        " · · ·   · · ·  ",
        "· · · · · · · · ",
        " · · · · · · ·  ",
        "· ·   · · · · · ",
        " · · · ·   · ·  ",
        "· · · · · · · · ",
        " · ·   · · · ·  ",
        "· · · · · · · · ",
        " · · · · · · ·  ",
        "                ",
    ],
};
pub const ENEMY_SHIELDED_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "      ▄▄▄▄      ",
        "    ▐▐████▌▌    ",
        "   ▐▐██████▌▌   ",
        "  ▐▐████████▌▌  ",
        " ▐▐██████▣███▌▌ ",
        "▐▐████████████▌▌",
        "▐▐████████████▌▌",
        "▐▐████████████▌▌",
        "▐▐████████████▌▌",
        "▐▐████████████▌▌",
        " ▐▐██████████▌▌ ",
        "  ▐▐████████▌▌  ",
        "   ▀▀████████▀  ",
        "    ▀▀██████▀   ",
        "      ▀████▀    ",
        "       ▀▀▀▀     ",
    ],
};
pub const ENEMY_HEALER_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "      ▄▄▄▄      ",
        "     ▄████▄     ",
        "    ▄██████▄    ",
        "   ▄████████▄   ",
        "  ▐█✚██████✚█▌  ",
        " ▐█████████████▌",
        "████████████████",
        "████████████████",
        "████████████████",
        " ▀█████████████▀",
        "  ▀███████████▀ ",
        "   ▀█████████▀  ",
        "    ▀███████▀   ",
        "     ▀█████▀    ",
        "      ▀▀▀▀▀     ",
        "                ",
    ],
};
pub const ENEMY_SNEAK_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "       ◆◆       ",
        "      ◆░░◆      ",
        "     ◆░░░░◆     ",
        "    ◆░░░░░░◆    ",
        "   ◆░░░░░░░░◆   ",
        "  ◆░░░░░░░░░░◆  ",
        " ◆░░░░░░░░░░░░◆ ",
        "◆░░░░░░░░░░░░░░◆",
        "◆░░░░░░░░░░░░░░◆",
        " ◆░░░░░░░░░░░░◆ ",
        "  ◆░░░░░░░░░░◆  ",
        "   ◆░░░░░░░░◆   ",
        "    ◆░░░░░░◆    ",
        "     ◆░░░░◆     ",
        "      ◆░░◆      ",
        "       ◆◆       ",
    ],
};

pub const IMPACT_Z0: Sprite = Sprite {
    w: 2,
    h: 2,
    rows: &["++", "++"],
};

pub const IMPACT_Z1: Sprite = Sprite {
    w: 4,
    h: 4,
    rows: &[" ✦  ", "✦✹✦ ", " ✦  ", "    "],
};
pub const IMPACT_Z2: Sprite = Sprite {
    w: 8,
    h: 8,
    rows: &[
        "   ✦    ",
        " ✦✹✦✹✦ ",
        "  ✹✦✹  ",
        " ✦✹✦✹✦ ",
        "   ✦    ",
        "        ",
        "        ",
        "        ",
    ],
};
pub const IMPACT_Z3: Sprite = Sprite {
    w: 12,
    h: 12,
    rows: &[
        "     ✦      ",
        "   ✦✹✦✹✦   ",
        "  ✹✦✹✦✹✦  ",
        " ✦✹✦✹✦✹✦ ",
        "  ✹✦✹✦✹✦  ",
        "   ✦✹✦✹✦   ",
        "     ✦      ",
        "            ",
        "            ",
        "            ",
        "            ",
        "            ",
    ],
};
pub const IMPACT_Z4: Sprite = Sprite {
    w: 16,
    h: 16,
    rows: &[
        "       ✦        ",
        "     ✦✹✦✹✦     ",
        "    ✹✦✹✦✹✦    ",
        "   ✦✹✦✹✦✹✦   ",
        "  ✹✦✹✦✹✦✹✦  ",
        "   ✦✹✦✹✦✹✦   ",
        "    ✹✦✹✦✹✦    ",
        "     ✦✹✦✹✦     ",
        "       ✦        ",
        "                ",
        "                ",
        "                ",
        "                ",
        "                ",
        "                ",
        "                ",
    ],
};

// ---------------------------
// Projéteis (um glifo, renderizado no centro do tile)
// ---------------------------

pub const GLYPH_PROJECTILE_BASIC: &str = "•";
pub const GLYPH_PROJECTILE_SNIPER: &str = "◆";
pub const GLYPH_PROJECTILE_RAPID: &str = "·";
pub const GLYPH_PROJECTILE_CANNON: &str = "●";
pub const GLYPH_PROJECTILE_TESLA: &str = "x";
pub const GLYPH_PROJECTILE_FROST: &str = "*";

// Traçantes direcionais (sniper)
pub const GLYPH_TRACER_H: &str = "━";
pub const GLYPH_TRACER_V: &str = "┃";
pub const GLYPH_TRACER_D1: &str = "╲";
pub const GLYPH_TRACER_D2: &str = "╱";

// ---------------------------
// Partículas (6 estágios p/ trilhas, 4 p/ restantes)
// idx 0 = forte, último = fraco
// ---------------------------

pub const TRAIL_BASIC: [&str; 6] = ["•", "∘", "·", "∙", " ", " "];
pub const TRAIL_SNIPER: [&str; 6] = ["━", "─", "╌", "·", " ", " "];
pub const TRAIL_RAPID: [&str; 6] = ["▪", "•", "·", "∙", " ", " "];
pub const TRAIL_CANNON: [&str; 6] = ["●", "◍", "○", "◌", "∘", " "];
pub const TRAIL_TESLA: [&str; 6] = ["╋", "╬", "╌", "·", "∙", " "];
pub const TRAIL_FROST: [&str; 6] = ["◆", "✦", "░", "·", "∙", " "];

pub const SPARK: [&str; 4] = ["✦", "✧", "∙", "·"];
pub const SMOKE: [&str; 4] = ["▓", "▒", "░", " "];
pub const ARC: [&str; 4] = ["╋", "╬", "╌", "·"];
pub const BOLT: [&str; 4] = ["╬", "╋", "╌", "∙"];
pub const SHARD: [&str; 4] = ["◆", "▲", "∙", "·"];
pub const FROST: [&str; 4] = ["◆", "✦", "░", "·"];
pub const WAVE: [&str; 4] = ["◜", "◝", "◞", "◟"];

// ---------------------------
// Dispatch: escolher sprite por zoom
// ---------------------------

pub fn tower_sprite(kind: TowerKind, zoom: u16) -> Sprite {
    match (kind, zoom.clamp(0, 4)) {
        (TowerKind::Basic, 0) => TOWER_BASIC_Z0,
        (TowerKind::Basic, 1) => TOWER_BASIC_Z1,
        (TowerKind::Basic, 2) => TOWER_BASIC_Z2,
        (TowerKind::Basic, 3) => TOWER_BASIC_Z3,
        (TowerKind::Basic, 4) => TOWER_BASIC_Z4,

        (TowerKind::Sniper, 0) => TOWER_SNIPER_Z0,
        (TowerKind::Sniper, 1) => TOWER_SNIPER_Z1,
        (TowerKind::Sniper, 2) => TOWER_SNIPER_Z2,
        (TowerKind::Sniper, 3) => TOWER_SNIPER_Z3,
        (TowerKind::Sniper, 4) => TOWER_SNIPER_Z4,

        (TowerKind::Rapid, 0) => TOWER_RAPID_Z0,
        (TowerKind::Rapid, 1) => TOWER_RAPID_Z1,
        (TowerKind::Rapid, 2) => TOWER_RAPID_Z2,
        (TowerKind::Rapid, 3) => TOWER_RAPID_Z3,
        (TowerKind::Rapid, 4) => TOWER_RAPID_Z4,

        (TowerKind::Cannon, 0) => TOWER_CANNON_Z0,
        (TowerKind::Cannon, 1) => TOWER_CANNON_Z1,
        (TowerKind::Cannon, 2) => TOWER_CANNON_Z2,
        (TowerKind::Cannon, 3) => TOWER_CANNON_Z3,
        (TowerKind::Cannon, 4) => TOWER_CANNON_Z4,

        (TowerKind::Tesla, 0) => TOWER_TESLA_Z0,
        (TowerKind::Tesla, 1) => TOWER_TESLA_Z1,
        (TowerKind::Tesla, 2) => TOWER_TESLA_Z2,
        (TowerKind::Tesla, 3) => TOWER_TESLA_Z3,
        (TowerKind::Tesla, 4) => TOWER_TESLA_Z4,

        (TowerKind::Frost, 0) => TOWER_FROST_Z0,
        (TowerKind::Frost, 1) => TOWER_FROST_Z1,
        (TowerKind::Frost, 2) => TOWER_FROST_Z2,
        (TowerKind::Frost, 3) => TOWER_FROST_Z3,
        (TowerKind::Frost, 4) => TOWER_FROST_Z4,

        _ => TOWER_BASIC_Z1,
    }
}

pub fn enemy_sprite(kind: EnemyKind, zoom: u16) -> Sprite {
    match zoom.clamp(0, 4) {
        0 => ENEMY_Z0,
        1 => match kind {
            EnemyKind::Runner => ENEMY_RUNNER_Z1,
            EnemyKind::Tank => ENEMY_TANK_Z1,
            EnemyKind::Swarm => ENEMY_SWARM_Z1,
            EnemyKind::Healer => ENEMY_HEALER_Z1,
            EnemyKind::Shielded => ENEMY_SHIELDED_Z1,
            EnemyKind::Sneak => ENEMY_SNEAK_Z1,
        },
        2 => match kind {
            EnemyKind::Runner => ENEMY_RUNNER_Z2,
            EnemyKind::Tank => ENEMY_TANK_Z2,
            EnemyKind::Swarm => ENEMY_SWARM_Z2,
            EnemyKind::Healer => ENEMY_HEALER_Z2,
            EnemyKind::Shielded => ENEMY_SHIELDED_Z2,
            EnemyKind::Sneak => ENEMY_SNEAK_Z2,
        },
        3 => match kind {
            EnemyKind::Runner => ENEMY_RUNNER_Z3,
            EnemyKind::Tank => ENEMY_TANK_Z3,
            EnemyKind::Swarm => ENEMY_SWARM_Z3,
            EnemyKind::Healer => ENEMY_HEALER_Z3,
            EnemyKind::Shielded => ENEMY_SHIELDED_Z3,
            EnemyKind::Sneak => ENEMY_SNEAK_Z3,
        },
        4 => match kind {
            EnemyKind::Runner => ENEMY_RUNNER_Z4,
            EnemyKind::Tank => ENEMY_TANK_Z4,
            EnemyKind::Swarm => ENEMY_SWARM_Z4,
            EnemyKind::Healer => ENEMY_HEALER_Z4,
            EnemyKind::Shielded => ENEMY_SHIELDED_Z4,
            EnemyKind::Sneak => ENEMY_SNEAK_Z4,
        },
        _ => ENEMY_RUNNER_Z1,
    }
}

pub fn impact_sprite(zoom: u16) -> Sprite {
    match zoom.clamp(0, 4) {
        0 => IMPACT_Z0,
        1 => IMPACT_Z1,
        2 => IMPACT_Z2,
        3 => IMPACT_Z3,
        4 => IMPACT_Z4,
        _ => IMPACT_Z1,
    }
}
