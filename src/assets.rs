// Assets Unicode (sem fallback ASCII)
//
// Regras:
// - tudo Unicode
// - evitar glifos que costumam virar emoji (ex.: caveira ☠, carinhas, etc.)
// - map usa tile_w=2, mas aqui os glifos são de 1 coluna e a UI completa com um espaço.

// Terreno
pub const GLYPH_GRASS: &str = "·";
pub const GLYPH_PATH: &str = "▒";

// Unidades
pub const GLYPH_TOWER_BASIC: &str = "╬";
pub const GLYPH_TOWER_SNIPER: &str = "◎";
pub const GLYPH_TOWER_RAPID: &str = "▣";
pub const GLYPH_ENEMY: &str = "▲";

// Disparo
pub const GLYPH_PROJECTILE_BASIC: &str = "⠂"; // Braille (não-emoji, bem leve)
pub const GLYPH_PROJECTILE_SNIPER: &str = "⠁";
pub const GLYPH_PROJECTILE_RAPID: &str = "⠄";

// VFX: impacto e partículas
pub const GLYPH_IMPACT_BIG: &str = "⟐";

// Partículas (quanto menor o TTL, mais "fraco")
pub const TRAIL: [&str; 4] = ["⠂", "⠄", "⠆", "⠁"]; // trilha
pub const SPARK: [&str; 4] = ["⠒", "⠖", "⠶", "⠷"]; // fagulhas
pub const SMOKE: [&str; 4] = ["░", "▒", "▓", "█"]; // "fumaça" (densidade)
