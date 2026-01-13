// Assets Unicode (sprites escaláveis por zoom)
//
// Objetivo:
// - substituir ASCII feio por sprites Unicode em "LOD" (level of detail)
// - manter tudo em largura 1 (evitar emojis / glifos double-width)
// - permitir zoom com tile_w/tile_h variáveis (o mapa "cresce" em X e Y)
//
// Observação: sprites usam espaço como transparência.

use crate::app::TowerKind;

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
// Sprites: Torres (1..=4)
// tile_w = 2*zoom, tile_h = zoom
// ---------------------------

// BASIC
pub const TOWER_BASIC_Z1: Sprite = Sprite {
    w: 2,
    h: 1,
    rows: &["▓▓"],
};
pub const TOWER_BASIC_Z2: Sprite = Sprite {
    w: 4,
    h: 2,
    rows: &[" ▄▄ ", "▄██▄"],
};
pub const TOWER_BASIC_Z3: Sprite = Sprite {
    w: 6,
    h: 3,
    rows: &[" ▄██▄ ", "▄████▄", "▀████▀"],
};
pub const TOWER_BASIC_Z4: Sprite = Sprite {
    w: 8,
    h: 4,
    rows: &["  ▄██▄  ", " ▄████▄ ", "▄██████▄", "▀██████▀"],
};

// SNIPER
pub const TOWER_SNIPER_Z1: Sprite = Sprite {
    w: 2,
    h: 1,
    rows: &["╞═"],
};
pub const TOWER_SNIPER_Z2: Sprite = Sprite {
    w: 4,
    h: 2,
    rows: &["╞═══", "└┬──"],
};
pub const TOWER_SNIPER_Z3: Sprite = Sprite {
    w: 6,
    h: 3,
    rows: &["╞═════", "└┬──┐ ", " └──┘ "],
};
pub const TOWER_SNIPER_Z4: Sprite = Sprite {
    w: 8,
    h: 4,
    rows: &["╞═══════", "└┬───┐  ", " ├───┤  ", " └───┘  "],
};

// RAPID
pub const TOWER_RAPID_Z1: Sprite = Sprite {
    w: 2,
    h: 1,
    rows: &["╔╗"],
};
pub const TOWER_RAPID_Z2: Sprite = Sprite {
    w: 4,
    h: 2,
    rows: &["╔══╗", "╚╦╦╝"],
};
pub const TOWER_RAPID_Z3: Sprite = Sprite {
    w: 6,
    h: 3,
    rows: &["╔════╗", "╚╦╦╦╝ ", " └┴┘  "],
};
pub const TOWER_RAPID_Z4: Sprite = Sprite {
    w: 8,
    h: 4,
    rows: &["╔══════╗", "╚╦╦╦╦╦╝ ", " └┴┴┴┴┘ ", "   ┴┴   "],
};

// CANNON
pub const TOWER_CANNON_Z1: Sprite = Sprite {
    w: 2,
    h: 1,
    rows: &["◉◉"],
};
pub const TOWER_CANNON_Z2: Sprite = Sprite {
    w: 4,
    h: 2,
    rows: &["◉══◉", " ▀▀ "],
};
pub const TOWER_CANNON_Z3: Sprite = Sprite {
    w: 6,
    h: 3,
    rows: &["◉════◉", " ▄██▄ ", "  ▀▀  "],
};
pub const TOWER_CANNON_Z4: Sprite = Sprite {
    w: 8,
    h: 4,
    rows: &["◉══════◉", " ▄████▄ ", " ▄████▄ ", "  ▀▀▀▀  "],
};

// TESLA
pub const TOWER_TESLA_Z1: Sprite = Sprite {
    w: 2,
    h: 1,
    rows: &["╳╳"],
};
pub const TOWER_TESLA_Z2: Sprite = Sprite {
    w: 4,
    h: 2,
    rows: &["╱╲╱╲", "╲╱╲╱"],
};
pub const TOWER_TESLA_Z3: Sprite = Sprite {
    w: 6,
    h: 3,
    rows: &["╱╲╱╲╱╲", "╲╱╲╱╲╱", " └──┘ "],
};
pub const TOWER_TESLA_Z4: Sprite = Sprite {
    w: 8,
    h: 4,
    rows: &["╱╲╱╲╱╲╱╲", "╲╱╲╱╲╱╲╱", "  └──┘  ", "  ┌──┐  "],
};

// FROST
pub const TOWER_FROST_Z1: Sprite = Sprite {
    w: 2,
    h: 1,
    rows: &["◇◇"],
};
pub const TOWER_FROST_Z2: Sprite = Sprite {
    w: 4,
    h: 2,
    rows: &["◇▙▟◇", " ▀▀ "],
};
pub const TOWER_FROST_Z3: Sprite = Sprite {
    w: 6,
    h: 3,
    rows: &["◇▟██▙◇", " ▜██▛ ", "  ▀▀  "],
};
pub const TOWER_FROST_Z4: Sprite = Sprite {
    w: 8,
    h: 4,
    rows: &["◇▟████▙◇", " ▜████▛ ", " ▟████▙ ", "  ▀▀▀▀  "],
};

pub fn tower_sprite(kind: TowerKind, zoom: u16) -> Sprite {
    match (kind, zoom.clamp(1, 4)) {
        (TowerKind::Basic, 1) => TOWER_BASIC_Z1,
        (TowerKind::Basic, 2) => TOWER_BASIC_Z2,
        (TowerKind::Basic, 3) => TOWER_BASIC_Z3,
        (TowerKind::Basic, _) => TOWER_BASIC_Z4,

        (TowerKind::Sniper, 1) => TOWER_SNIPER_Z1,
        (TowerKind::Sniper, 2) => TOWER_SNIPER_Z2,
        (TowerKind::Sniper, 3) => TOWER_SNIPER_Z3,
        (TowerKind::Sniper, _) => TOWER_SNIPER_Z4,

        (TowerKind::Rapid, 1) => TOWER_RAPID_Z1,
        (TowerKind::Rapid, 2) => TOWER_RAPID_Z2,
        (TowerKind::Rapid, 3) => TOWER_RAPID_Z3,
        (TowerKind::Rapid, _) => TOWER_RAPID_Z4,

        (TowerKind::Cannon, 1) => TOWER_CANNON_Z1,
        (TowerKind::Cannon, 2) => TOWER_CANNON_Z2,
        (TowerKind::Cannon, 3) => TOWER_CANNON_Z3,
        (TowerKind::Cannon, _) => TOWER_CANNON_Z4,

        (TowerKind::Tesla, 1) => TOWER_TESLA_Z1,
        (TowerKind::Tesla, 2) => TOWER_TESLA_Z2,
        (TowerKind::Tesla, 3) => TOWER_TESLA_Z3,
        (TowerKind::Tesla, _) => TOWER_TESLA_Z4,

        (TowerKind::Frost, 1) => TOWER_FROST_Z1,
        (TowerKind::Frost, 2) => TOWER_FROST_Z2,
        (TowerKind::Frost, 3) => TOWER_FROST_Z3,
        (TowerKind::Frost, _) => TOWER_FROST_Z4,
    }
}

// ---------------------------
// Sprites: Inimigo / Impacto
// ---------------------------

pub const ENEMY_Z1: Sprite = Sprite {
    w: 2,
    h: 1,
    rows: &["╬╬"],
};
pub const ENEMY_Z2: Sprite = Sprite {
    w: 4,
    h: 2,
    rows: &["▟██▙", "▜██▛"],
};
pub const ENEMY_Z3: Sprite = Sprite {
    w: 6,
    h: 3,
    rows: &["▟████▙", "▜█▛▜█▛", " ▀██▀ "],
};
pub const ENEMY_Z4: Sprite = Sprite {
    w: 8,
    h: 4,
    rows: &["▟██████▙", "▜██▛▜██▛", " ▜████▛ ", "  ▀██▀  "],
};

pub fn enemy_sprite(zoom: u16) -> Sprite {
    match zoom.clamp(1, 4) {
        1 => ENEMY_Z1,
        2 => ENEMY_Z2,
        3 => ENEMY_Z3,
        _ => ENEMY_Z4,
    }
}

pub const IMPACT_Z1: Sprite = Sprite {
    w: 2,
    h: 1,
    rows: &["✸✹"],
};
pub const IMPACT_Z2: Sprite = Sprite {
    w: 4,
    h: 2,
    rows: &["✸✹✸✹", "✹✸✹✸"],
};
pub const IMPACT_Z3: Sprite = Sprite {
    w: 6,
    h: 3,
    rows: &[" ✸✹✸✹ ", "✹✸✹✸✹✸", " ✸✹✸✹ "],
};
pub const IMPACT_Z4: Sprite = Sprite {
    w: 8,
    h: 4,
    rows: &["  ✸✹✸✹  ", " ✹✸✹✸✹✸ ", " ✸✹✸✹✸✹ ", "  ✹✸✹✸  "],
};

pub fn impact_sprite(zoom: u16) -> Sprite {
    match zoom.clamp(1, 4) {
        1 => IMPACT_Z1,
        2 => IMPACT_Z2,
        3 => IMPACT_Z3,
        _ => IMPACT_Z4,
    }
}

// ---------------------------
// Disparo / Partículas (1 célula)
// ---------------------------

pub const GLYPH_PROJECTILE_BASIC: &str = "➤";
pub const GLYPH_PROJECTILE_SNIPER: &str = "⟡";
pub const GLYPH_PROJECTILE_RAPID: &str = "✦";
pub const GLYPH_PROJECTILE_CANNON: &str = "⬤";
pub const GLYPH_PROJECTILE_TESLA: &str = "⚡";
pub const GLYPH_PROJECTILE_FROST: &str = "❄";
pub const GLYPH_PROJECTILE_STACK: &str = "≣";

pub const TRAIL_BASIC: [&str; 4] = ["•", "∙", "·", "·"];
pub const TRAIL_SNIPER: [&str; 4] = ["╍", "┄", "┈", "·"];
pub const TRAIL_RAPID: [&str; 4] = ["⋆", "✶", "·", "·"];
pub const TRAIL_CANNON: [&str; 4] = ["●", "◍", "·", "·"];
pub const TRAIL_TESLA: [&str; 4] = ["⚡", "╱", "╲", "·"];
pub const TRAIL_FROST: [&str; 4] = ["❅", "❆", "·", "·"];
pub const SPARK: [&str; 4] = ["✹", "✸", "✷", "·"];
pub const SMOKE: [&str; 4] = ["▓", "▒", "░", "∙"];
pub const ARC: [&str; 4] = ["⚡", "╱", "╲", "·"];
pub const SHARD: [&str; 4] = ["✷", "✶", "✵", "·"];
pub const BOLT: [&str; 4] = ["⚡", "⟋", "⟍", "·"];
pub const FROST: [&str; 4] = ["❄", "✳", "✱", "·"];
pub const WAVE: [&str; 4] = ["✺", "✹", "✷", "·"];
pub const PULSE: [&str; 4] = ["◎", "◉", "○", "·"];
pub const NEEDLE: [&str; 4] = ["✦", "✧", "·", "·"];
pub const SPRAY: [&str; 4] = ["※", "✺", "·", "·"];
pub const EMBER: [&str; 4] = ["⟁", "⟡", "·", "·"];
pub const STATIC: [&str; 4] = ["≋", "≈", "·", "·"];
pub const FLAKE: [&str; 4] = ["❋", "✳", "·", "·"];
