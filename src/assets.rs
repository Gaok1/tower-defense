// Assets Unicode (sem fallback ASCII)
//
// Regras:
// - tudo Unicode
// - evitar glifos que costumam virar emoji (ex.: caveira ☠, carinhas, etc.)
// - map usa tile_w=2, mas aqui os glifos são de 1 coluna e a UI completa com um espaço.

#[derive(Clone, Copy)]
pub struct Glyph2 {
    pub left: &'static str,
    pub right: &'static str,
}

// Terreno
pub const GLYPH_GOAL: Glyph2 = Glyph2 {
    left: "⛬",
    right: "⛬",
};
pub const GLYPH_PATH: [Glyph2; 2] = [
    Glyph2 {
        left: "▓",
        right: "▓",
    },
    Glyph2 {
        left: "▓",
        right: "█",
    },
];
pub const GLYPH_GRASS: [Glyph2; 4] = [
    Glyph2 {
        left: "░",
        right: "░",
    },
    Glyph2 {
        left: "░",
        right: "▒",
    },
    Glyph2 {
        left: "▒",
        right: "░",
    },
    Glyph2 {
        left: "░",
        right: "░",
    },
];

// Unidades
pub const GLYPH_TOWER_BASIC: Glyph2 = Glyph2 {
    left: "╔",
    right: "╗",
};
pub const GLYPH_TOWER_SNIPER: Glyph2 = Glyph2 {
    left: "◥",
    right: "◤",
};
pub const GLYPH_TOWER_RAPID: Glyph2 = Glyph2 {
    left: "╟",
    right: "╢",
};
pub const GLYPH_TOWER_CANNON: Glyph2 = Glyph2 {
    left: "╦",
    right: "╦",
};
pub const GLYPH_TOWER_TESLA: Glyph2 = Glyph2 {
    left: "╩",
    right: "╩",
};
pub const GLYPH_TOWER_FROST: Glyph2 = Glyph2 {
    left: "╣",
    right: "╠",
};
pub const GLYPH_ENEMY: Glyph2 = Glyph2 {
    left: "◁",
    right: "▷",
};

// Disparo
pub const GLYPH_PROJECTILE_BASIC: &str = "⠂"; // Braille (não-emoji, bem leve)
pub const GLYPH_PROJECTILE_SNIPER: &str = "⠁";
pub const GLYPH_PROJECTILE_RAPID: &str = "⠄";
pub const GLYPH_PROJECTILE_CANNON: &str = "⠶";
pub const GLYPH_PROJECTILE_TESLA: &str = "⠲";
pub const GLYPH_PROJECTILE_FROST: &str = "⠴";

// VFX: impacto e partículas
pub const GLYPH_IMPACT_BIG: Glyph2 = Glyph2 {
    left: "⟐",
    right: "⟐",
};

// Partículas (quanto menor o TTL, mais "fraco")
pub const TRAIL: [&str; 4] = ["⠂", "⠄", "⠆", "⠁"]; // trilha
pub const SPARK: [&str; 4] = ["⠒", "⠖", "⠶", "⠷"]; // fagulhas
pub const SMOKE: [&str; 4] = ["░", "▒", "▓", "█"]; // "fumaça" (densidade)
pub const ARC: [&str; 4] = ["⠈", "⠘", "⠸", "⠹"]; // faísca elétrica
pub const SHARD: [&str; 4] = ["⠐", "⠠", "⠤", "⠦"]; // estilhaço/gelado
