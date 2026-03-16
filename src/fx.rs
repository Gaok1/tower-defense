use crate::app::{Enemy, EnemyKind, MapViewport, TowerKind};
use crate::assets;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
};

const MAX_FX_ACTIVE: usize = 5000;
const MAX_PRIMITIVES_PER_FRAME: usize = 2500;
const MAX_ARC_LIGHTNING: u16 = 20;
const MAX_IMPACT_RING: u16 = 30;
const MAX_DUST: u16 = 100;
const MAX_PROJECTILE: u16 = 2000;

const MAX_DUST_CELLS: usize = 6;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Vec2i {
    pub x: i16,
    pub y: i16,
}

impl Vec2i {
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxKind {
    MuzzleFlash,
    TracerLine,
    Projectile,
    ImpactCross,
    ImpactRing,
    Dust,
    ArcLightning,
    TargetFlash,
    Shatter,
    StatusOverlay,
    HitFlash,
    EnemyDeath,
}

impl FxKind {
    pub const COUNT: usize = 12;

    pub fn index(self) -> usize {
        match self {
            FxKind::MuzzleFlash => 0,
            FxKind::TracerLine => 1,
            FxKind::Projectile => 2,
            FxKind::ImpactCross => 3,
            FxKind::ImpactRing => 4,
            FxKind::Dust => 5,
            FxKind::ArcLightning => 6,
            FxKind::TargetFlash => 7,
            FxKind::Shatter => 8,
            FxKind::StatusOverlay => 9,
            FxKind::HitFlash => 10,
            FxKind::EnemyDeath => 11,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StatusOverlayKind {
    Frost,
}

#[derive(Debug, Clone, Copy)]
pub enum FxData {
    None,
    Muzzle {
        kind: TowerKind,
        dir: Vec2i,
    },
    TracerLine {
        kind: TowerKind,
        from: Vec2i,
        to: Vec2i,
    },
    Projectile {
        kind: TowerKind,
        dir: Vec2i,
        glyph_head: &'static str,
        glyph_trail: &'static str,
    },
    ImpactCross {
        kind: TowerKind,
    },
    ImpactRing {
        max_radius: u8,
    },
    Dust {
        count: u8,
        cells: [Vec2i; MAX_DUST_CELLS],
    },
    ArcLightning {
        from: Vec2i,
        to: Vec2i,
        segments_max: u8,
    },
    StatusOverlay {
        target: usize,
        kind: StatusOverlayKind,
    },
    HitFlash {
        kind: EnemyKind,
    },
    EnemyDeath {
        kind: EnemyKind,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct FxEntity {
    pub kind: FxKind,
    pub pos: Vec2i,
    pub ttl: u8,
    pub age: u8,
    pub priority: u8,
    pub seed: u32,
    pub data: FxData,
}

impl FxEntity {
    fn empty() -> Self {
        Self {
            kind: FxKind::MuzzleFlash,
            pos: Vec2i::default(),
            ttl: 0,
            age: 0,
            priority: 0,
            seed: 0,
            data: FxData::None,
        }
    }
}

#[derive(Debug, Clone)]
struct FxSlot {
    entity: FxEntity,
    active: bool,
}

impl FxSlot {
    fn new() -> Self {
        Self {
            entity: FxEntity::empty(),
            active: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FxConfig {
    max_primitives_per_frame: usize,
    max_active: usize,
    max_by_kind: [u16; FxKind::COUNT],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FxFrameStats {
    pub primitives_drawn: u16,
    pub culled_by_budget: u16,
    pub culled_by_kind: u16,
    pub active_by_kind: [u16; FxKind::COUNT],
}

#[derive(Debug, Clone, Copy)]
enum FxLod {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct FxManager {
    slots: Vec<FxSlot>,
    free_list: Vec<usize>,
    buckets: Vec<Vec<usize>>,
    config: FxConfig,
    stats: FxFrameStats,
    active_counts: [u16; FxKind::COUNT],
    spawn_culled_by_kind: u16,
}

impl FxManager {
    pub fn new() -> Self {
        let mut slots = Vec::with_capacity(MAX_FX_ACTIVE);
        let mut free_list = Vec::with_capacity(MAX_FX_ACTIVE);
        for idx in 0..MAX_FX_ACTIVE {
            slots.push(FxSlot::new());
            free_list.push(idx);
        }

        let mut buckets = Vec::with_capacity(101);
        for _ in 0..=100 {
            buckets.push(Vec::new());
        }

        let mut max_by_kind = [0u16; FxKind::COUNT];
        max_by_kind[FxKind::ArcLightning.index()] = MAX_ARC_LIGHTNING;
        max_by_kind[FxKind::ImpactRing.index()] = MAX_IMPACT_RING;
        max_by_kind[FxKind::Dust.index()] = MAX_DUST;
        max_by_kind[FxKind::Projectile.index()] = MAX_PROJECTILE;
        max_by_kind[FxKind::HitFlash.index()] = 200;
        max_by_kind[FxKind::EnemyDeath.index()] = 100;

        Self {
            slots,
            free_list,
            buckets,
            config: FxConfig {
                max_primitives_per_frame: MAX_PRIMITIVES_PER_FRAME,
                max_active: MAX_FX_ACTIVE,
                max_by_kind,
            },
            stats: FxFrameStats::default(),
            active_counts: [0; FxKind::COUNT],
            spawn_culled_by_kind: 0,
        }
    }

    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            slot.active = false;
        }
        self.free_list.clear();
        for idx in 0..self.config.max_active {
            self.free_list.push(idx);
        }
        self.stats = FxFrameStats::default();
        self.active_counts = [0; FxKind::COUNT];
        self.spawn_culled_by_kind = 0;
    }

    pub fn tick(&mut self) {
        self.spawn_culled_by_kind = 0;
        for slot in &mut self.slots {
            if !slot.active {
                continue;
            }
            slot.entity.ttl = slot.entity.ttl.saturating_sub(1);
            slot.entity.age = slot.entity.age.saturating_add(1);
            if slot.entity.ttl == 0 {
                self.active_counts[slot.entity.kind.index()] =
                    self.active_counts[slot.entity.kind.index()].saturating_sub(1);
                slot.active = false;
            }
        }
        self.rebuild_free_list();
    }

    pub fn stats(&self) -> FxFrameStats {
        self.stats
    }

    pub fn spawn_muzzle(&mut self, kind: TowerKind, tower_pos: Vec2i, dir: Vec2i, seed: u32) {
        let profile = muzzle_profile(kind);
        let pos = Vec2i::new(tower_pos.x + dir.x, tower_pos.y + dir.y);
        let entity = FxEntity {
            kind: FxKind::MuzzleFlash,
            pos,
            ttl: profile.ttl,
            age: 0,
            priority: 90,
            seed,
            data: FxData::Muzzle { kind, dir },
        };
        self.spawn_entity(entity, true);
    }

    pub fn spawn_tracer_line(&mut self, kind: TowerKind, from: Vec2i, to: Vec2i, seed: u32) {
        let entity = FxEntity {
            kind: FxKind::TracerLine,
            pos: from,
            ttl: 1,
            age: 0,
            priority: 60,
            seed,
            data: FxData::TracerLine { kind, from, to },
        };
        self.spawn_entity(entity, false);
    }

    pub fn spawn_projectile(
        &mut self,
        kind: TowerKind,
        from: Vec2i,
        to: Vec2i,
        ttl: u8,
        seed: u32,
    ) -> Option<usize> {
        let dir = Vec2i::new((to.x - from.x).signum(), (to.y - from.y).signum());
        let (glyph_head, glyph_trail) = projectile_glyphs(kind);
        let entity = FxEntity {
            kind: FxKind::Projectile,
            pos: from,
            ttl,
            age: 0,
            priority: 80,
            seed,
            data: FxData::Projectile {
                kind,
                dir,
                glyph_head,
                glyph_trail,
            },
        };
        self.spawn_entity_with_id(entity, false)
    }

    pub fn update_projectile_pos(&mut self, id: usize, pos: Vec2i, dir: Vec2i) {
        if let Some(slot) = self.slots.get_mut(id) {
            if slot.active && slot.entity.kind == FxKind::Projectile {
                slot.entity.pos = pos;
                if let FxData::Projectile {
                    dir: stored_dir, ..
                } = &mut slot.entity.data
                {
                    *stored_dir = dir;
                }
            }
        }
    }

    pub fn despawn(&mut self, id: usize) {
        if let Some(slot) = self.slots.get_mut(id) {
            if slot.active {
                self.active_counts[slot.entity.kind.index()] =
                    self.active_counts[slot.entity.kind.index()].saturating_sub(1);
                slot.active = false;
                self.free_list.push(id);
            }
        }
    }

    pub fn spawn_impact_cross(&mut self, pos: Vec2i, kind: TowerKind, seed: u32) {
        let entity = FxEntity {
            kind: FxKind::ImpactCross,
            pos,
            ttl: 2,
            age: 0,
            priority: 100,
            seed,
            data: FxData::ImpactCross { kind },
        };
        self.spawn_entity(entity, false);
    }

    pub fn spawn_impact_ring(&mut self, pos: Vec2i, seed: u32) {
        let entity = FxEntity {
            kind: FxKind::ImpactRing,
            pos,
            ttl: 3,
            age: 0,
            priority: 100,
            seed,
            data: FxData::ImpactRing { max_radius: 2 },
        };
        self.spawn_entity(entity, false);
    }

    pub fn spawn_dust(&mut self, center: Vec2i, seed: u32) {
        let mut cells = [Vec2i::default(); MAX_DUST_CELLS];
        let mut count = 0u8;
        let offsets = [
            Vec2i::new(-1, 0),
            Vec2i::new(1, 0),
            Vec2i::new(0, -1),
            Vec2i::new(0, 1),
            Vec2i::new(-1, -1),
            Vec2i::new(1, 1),
        ];
        let mut seed_state = seed;
        for off in offsets {
            if count as usize >= MAX_DUST_CELLS {
                break;
            }
            seed_state = xorshift32(seed_state);
            if seed_state % 3 == 0 {
                continue;
            }
            cells[count as usize] = Vec2i::new(center.x + off.x, center.y + off.y);
            count += 1;
        }
        if count == 0 {
            count = 1;
            cells[0] = center;
        }
        let entity = FxEntity {
            kind: FxKind::Dust,
            pos: center,
            ttl: 3,
            age: 0,
            priority: 40,
            seed,
            data: FxData::Dust { count, cells },
        };
        self.spawn_entity(entity, false);
    }

    pub fn spawn_arc_lightning(&mut self, from: Vec2i, to: Vec2i, seed: u32) {
        let entity = FxEntity {
            kind: FxKind::ArcLightning,
            pos: from,
            ttl: 2,
            age: 0,
            priority: 80,
            seed,
            data: FxData::ArcLightning {
                from,
                to,
                segments_max: 12,
            },
        };
        if self.spawn_entity(entity, false).is_none() {
            self.spawn_target_flash(to, seed ^ 0x9E37);
        }
    }

    pub fn spawn_tesla_beam_fx(&mut self, from: Vec2i, to: Vec2i, seed1: u32, seed2: u32) {
        let arc1 = FxEntity {
            kind: FxKind::ArcLightning,
            pos: from,
            ttl: 3,
            age: 0,
            priority: 85,
            seed: seed1,
            data: FxData::ArcLightning { from, to, segments_max: 12 },
        };
        self.spawn_entity(arc1, false);
        let arc2 = FxEntity {
            kind: FxKind::ArcLightning,
            pos: from,
            ttl: 1,
            age: 0,
            priority: 85,
            seed: seed2,
            data: FxData::ArcLightning { from, to, segments_max: 12 },
        };
        self.spawn_entity(arc2, false);
        self.spawn_target_flash(to, seed1 ^ 0xBEEF);
    }

    pub fn spawn_target_flash(&mut self, pos: Vec2i, seed: u32) {
        let entity = FxEntity {
            kind: FxKind::TargetFlash,
            pos,
            ttl: 1,
            age: 0,
            priority: 100,
            seed,
            data: FxData::None,
        };
        self.spawn_entity(entity, false);
    }

    pub fn spawn_shatter(&mut self, pos: Vec2i, seed: u32) {
        let entity = FxEntity {
            kind: FxKind::Shatter,
            pos,
            ttl: 2,
            age: 0,
            priority: 100,
            seed,
            data: FxData::None,
        };
        self.spawn_entity(entity, false);
    }

    pub fn spawn_status_overlay(&mut self, target: usize, ttl: u8, seed: u32) {
        for slot in &mut self.slots {
            if !slot.active {
                continue;
            }
            if let FxData::StatusOverlay { target: t, .. } = slot.entity.data {
                if t == target {
                    slot.entity.ttl = slot.entity.ttl.max(ttl);
                    slot.entity.seed = seed;
                    return;
                }
            }
        }
        let entity = FxEntity {
            kind: FxKind::StatusOverlay,
            pos: Vec2i::default(),
            ttl,
            age: 0,
            priority: 20,
            seed,
            data: FxData::StatusOverlay {
                target,
                kind: StatusOverlayKind::Frost,
            },
        };
        self.spawn_entity(entity, false);
    }

    pub fn spawn_hit_flash(&mut self, pos: Vec2i, kind: EnemyKind, seed: u32) {
        let entity = FxEntity {
            kind: FxKind::HitFlash,
            pos,
            ttl: 2,
            age: 0,
            priority: 90,
            seed,
            data: FxData::HitFlash { kind },
        };
        self.spawn_entity(entity, false);
    }

    pub fn spawn_enemy_death(&mut self, pos: Vec2i, kind: EnemyKind, seed: u32) {
        let entity = FxEntity {
            kind: FxKind::EnemyDeath,
            pos,
            ttl: 4,
            age: 0,
            priority: 60,
            seed,
            data: FxData::EnemyDeath { kind },
        };
        self.spawn_entity(entity, true);
    }

    pub fn render(
        &mut self,
        buf: &mut Buffer,
        area: Rect,
        viewport: MapViewport,
        zoom: u16,
        enemies: &[Enemy],
        path: &[(u16, u16)],
    ) -> FxFrameStats {
        let lod = lod_from_zoom(zoom);
        self.stats = FxFrameStats::default();
        self.stats.culled_by_kind = self.spawn_culled_by_kind;
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        for (idx, slot) in self.slots.iter().enumerate() {
            if !slot.active {
                continue;
            }
            let kind = slot.entity.kind;
            self.stats.active_by_kind[kind.index()] += 1;
            let bucket = &mut self.buckets[slot.entity.priority as usize];
            bucket.push(idx);
        }

        let mut budget = self.config.max_primitives_per_frame as i32;
        for priority in (0..=100).rev() {
            let bucket = &self.buckets[priority];
            for &idx in bucket {
                if budget <= 0 {
                    self.stats.culled_by_budget += 1;
                    continue;
                }
                let used = self.render_entity(
                    &self.slots[idx].entity,
                    lod,
                    zoom,
                    buf,
                    area,
                    viewport,
                    enemies,
                    path,
                    &mut budget,
                );
                self.stats.primitives_drawn += used as u16;
            }
        }

        self.stats
    }

    fn render_entity(
        &self,
        entity: &FxEntity,
        lod: FxLod,
        zoom: u16,
        buf: &mut Buffer,
        area: Rect,
        viewport: MapViewport,
        enemies: &[Enemy],
        path: &[(u16, u16)],
        budget: &mut i32,
    ) -> u16 {
        let mut used = 0u16;
        match entity.kind {
            FxKind::MuzzleFlash => {
                let FxData::Muzzle { kind, .. } = entity.data else {
                    return 0;
                };
                let profile = muzzle_profile(kind);
                let glyph = if entity.age == 0 || profile.glyph_fade.is_none() {
                    profile.glyph_main
                } else {
                    profile.glyph_fade.unwrap_or(profile.glyph_main)
                };
                used += draw_fx_cell(
                    entity.pos,
                    glyph,
                    tower_kind_color(kind),
                    Modifier::BOLD,
                    buf,
                    area,
                    viewport,
                    budget,
                );
            }
            FxKind::TracerLine => {
                let FxData::TracerLine { kind, from, to } = entity.data else {
                    return 0;
                };
                let color = tower_kind_color(kind);
                let glyph = line_glyph(from, to);

                if let (Some(a), Some(b)) = (
                    map_to_screen(from, area, viewport),
                    map_to_screen(to, area, viewport),
                ) {
                    if a != b {
                        used += draw_screen_line(a, b, color, Modifier::BOLD, buf, area, budget);
                    } else {
                        used += draw_screen_cell(
                            a.0,
                            a.1,
                            glyph,
                            color,
                            Modifier::BOLD,
                            buf,
                            area,
                            budget,
                        );
                    }
                    return used;
                }

                let points = line_points(from, to);
                let mut first: Option<(u16, u16)> = None;
                let mut last: Option<(u16, u16)> = None;
                for p in points {
                    if let Some(s) = map_to_screen(p, area, viewport) {
                        if first.is_none() {
                            first = Some(s);
                        }
                        last = Some(s);
                    }
                }

                match (first, last) {
                    (Some(a), Some(b)) if a != b => {
                        used += draw_screen_line(a, b, color, Modifier::BOLD, buf, area, budget);
                    }
                    (Some(a), _) => {
                        used += draw_screen_cell(
                            a.0,
                            a.1,
                            glyph,
                            color,
                            Modifier::BOLD,
                            buf,
                            area,
                            budget,
                        );
                    }
                    _ => {}
                }
            }
            FxKind::Projectile => {
                let FxData::Projectile {
                    kind,
                    dir,
                    glyph_head,
                    glyph_trail,
                } = entity.data
                else {
                    return 0;
                };
                used += draw_fx_cell(
                    entity.pos,
                    glyph_head,
                    tower_kind_color(kind),
                    Modifier::BOLD,
                    buf,
                    area,
                    viewport,
                    budget,
                );
                if *budget <= 0 {
                    return used;
                }
                if !matches!(lod, FxLod::Low) {
                    let trail_pos = Vec2i::new(entity.pos.x - dir.x, entity.pos.y - dir.y);
                    used += draw_cell(
                        trail_pos,
                        glyph_trail,
                        tower_kind_color(kind),
                        Modifier::DIM,
                        buf,
                        area,
                        viewport,
                        budget,
                    );
                    if *budget <= 0 {
                        return used;
                    }
                    if kind == TowerKind::Rapid {
                        let trail_pos2 =
                            Vec2i::new(entity.pos.x - dir.x * 2, entity.pos.y - dir.y * 2);
                        used += draw_cell(
                            trail_pos2,
                            glyph_trail,
                            tower_kind_color(kind),
                            Modifier::DIM,
                            buf,
                            area,
                            viewport,
                            budget,
                        );
                    }
                    if *budget <= 0 {
                        return used;
                    }
                    if kind == TowerKind::Cannon {
                        let side = Vec2i::new(-dir.y, dir.x);
                        let accent_pos = Vec2i::new(entity.pos.x + side.x, entity.pos.y + side.y);
                        used += draw_cell(
                            accent_pos,
                            "▓",
                            tower_kind_color(kind),
                            Modifier::DIM,
                            buf,
                            area,
                            viewport,
                            budget,
                        );
                    }
                }
            }
            FxKind::ImpactCross => {
                let glyph = if entity.age == 0 { "┼" } else { "▒" };
                let FxData::ImpactCross { kind } = entity.data else {
                    return 0;
                };
                used += draw_fx_cell(
                    entity.pos,
                    glyph,
                    tower_kind_color(kind),
                    Modifier::BOLD,
                    buf,
                    area,
                    viewport,
                    budget,
                );
            }
            FxKind::ImpactRing => {
                let FxData::ImpactRing { max_radius } = entity.data else {
                    return 0;
                };
                if entity.age == 0 {
                    used += draw_fx_cell(
                        entity.pos,
                        "█",
                        Color::LightRed,
                        Modifier::BOLD,
                        buf,
                        area,
                        viewport,
                        budget,
                    );
                } else if entity.age == 1 {
                    let radius = match lod {
                        FxLod::Low => 1,
                        FxLod::Medium => 1,
                        FxLod::High => max_radius.min(2) as i16,
                    };
                    used += draw_cardinals(
                        entity.pos,
                        radius,
                        "┼",
                        Color::LightRed,
                        buf,
                        area,
                        viewport,
                        budget,
                    );
                } else {
                    used += draw_fx_cell(
                        entity.pos,
                        "▒",
                        Color::LightRed,
                        Modifier::DIM,
                        buf,
                        area,
                        viewport,
                        budget,
                    );
                }
            }
            FxKind::Dust => {
                let FxData::Dust { count, cells } = entity.data else {
                    return 0;
                };
                let glyph = if entity.age == 0 { "▒" } else { "░" };
                let color = Color::DarkGray;
                for i in 0..count as usize {
                    if *budget <= 0 {
                        break;
                    }
                    used += draw_cell(
                        cells[i],
                        glyph,
                        color,
                        Modifier::DIM,
                        buf,
                        area,
                        viewport,
                        budget,
                    );
                }
            }
            FxKind::ArcLightning => {
                let FxData::ArcLightning {
                    from,
                    to,
                    segments_max,
                } = entity.data
                else {
                    return 0;
                };
                let mut points = line_points(from, to);
                let max_seg = match lod {
                    FxLod::Low => segments_max.min(6),
                    FxLod::Medium => segments_max.min(10),
                    FxLod::High => segments_max,
                } as usize;
                if points.len() > max_seg {
                    points.truncate(max_seg);
                }

                let tile = viewport.tile_w.min(viewport.tile_h) as i16;
                let jitter = (tile / 6).clamp(1, 2);
                let x_min = area.x as i16;
                let y_min = area.y as i16;
                let x_max = area.right().saturating_sub(1) as i16;
                let y_max = area.bottom().saturating_sub(1) as i16;

                let mut seed = entity.seed;
                for window in points.windows(2) {
                    if *budget <= 0 {
                        break;
                    }
                    let p0 = window[0];
                    let p1 = window[1];

                    let (mut x0, y0) = match map_to_screen(p0, area, viewport) {
                        Some(v) => (v.0 as i16, v.1 as i16),
                        None => continue,
                    };
                    let (x1, mut y1) = match map_to_screen(p1, area, viewport) {
                        Some(v) => (v.0 as i16, v.1 as i16),
                        None => continue,
                    };

                    seed = xorshift32(seed);
                    if seed % 3 == 0 {
                        let j = if seed % 2 == 0 { jitter } else { -jitter };
                        x0 = (x0 + j).clamp(x_min, x_max);
                    }
                    seed = xorshift32(seed);
                    if seed % 3 == 0 {
                        let j = if seed % 2 == 0 { jitter } else { -jitter };
                        y1 = (y1 + j).clamp(y_min, y_max);
                    }

                    let arc_color = if entity.age == 0 { Color::White } else { Color::Cyan };
                    used += draw_screen_line(
                        (x0 as u16, y0 as u16),
                        (x1 as u16, y1 as u16),
                        arc_color,
                        Modifier::BOLD,
                        buf,
                        area,
                        budget,
                    );
                }
            }
            FxKind::TargetFlash => {
                used += draw_fx_cell(
                    entity.pos,
                    "▓",
                    Color::Cyan,
                    Modifier::BOLD,
                    buf,
                    area,
                    viewport,
                    budget,
                );
            }
            FxKind::Shatter => {
                let glyph = if entity.age == 0 { "╳" } else { "░" };
                used += draw_fx_cell(
                    entity.pos,
                    glyph,
                    Color::Blue,
                    Modifier::BOLD,
                    buf,
                    area,
                    viewport,
                    budget,
                );
            }
            FxKind::StatusOverlay => {
                let FxData::StatusOverlay { target, .. } = entity.data else {
                    return 0;
                };
                if let Some(enemy) = enemies.get(target) {
                    if enemy.hp <= 0 || enemy.slow_ticks == 0 {
                        return 0;
                    }
                    let (ex, ey) = path.get(enemy.path_i).copied().unwrap_or((0, 0));
                    let glyph = if entity.age % 2 == 0 { "▓" } else { "▒" };
                    used += draw_fx_cell(
                        Vec2i::new(ex as i16, ey as i16),
                        glyph,
                        Color::Blue,
                        Modifier::DIM,
                        buf,
                        area,
                        viewport,
                        budget,
                    );
                }
            }
            FxKind::HitFlash => {
                let FxData::HitFlash { kind } = entity.data else {
                    return 0;
                };
                let color = enemy_kind_color(kind);
                let (glyph, modifier) = if entity.age == 0 {
                    ("▓", Modifier::BOLD)
                } else {
                    ("░", Modifier::DIM)
                };
                used += draw_fx_cell(
                    entity.pos,
                    glyph,
                    color,
                    modifier,
                    buf,
                    area,
                    viewport,
                    budget,
                );
            }
            FxKind::EnemyDeath => {
                let FxData::EnemyDeath { kind } = entity.data else {
                    return 0;
                };
                let color = enemy_kind_color(kind);
                let sprite = assets::enemy_sprite(kind, zoom);
                used += draw_fx_sprite_death(
                    entity.pos,
                    entity.age,
                    entity.ttl,
                    entity.seed,
                    sprite,
                    color,
                    buf,
                    area,
                    viewport,
                    budget,
                );
            }
        }
        used
    }

    fn spawn_entity(&mut self, entity: FxEntity, allow_replace: bool) -> Option<usize> {
        self.spawn_entity_with_id(entity, allow_replace)
    }

    fn spawn_entity_with_id(&mut self, entity: FxEntity, allow_replace: bool) -> Option<usize> {
        let kind_idx = entity.kind.index();
        if self.config.max_by_kind[kind_idx] > 0
            && self.active_counts[kind_idx] >= self.config.max_by_kind[kind_idx]
        {
            self.spawn_culled_by_kind = self.spawn_culled_by_kind.saturating_add(1);
            return None;
        }
        if let Some(idx) = self.free_list.pop() {
            self.slots[idx].entity = entity;
            self.slots[idx].active = true;
            self.active_counts[kind_idx] = self.active_counts[kind_idx].saturating_add(1);
            return Some(idx);
        }
        if !allow_replace {
            return None;
        }
        let mut candidate: Option<(usize, u8, u8)> = None;
        for (idx, slot) in self.slots.iter().enumerate() {
            if !slot.active {
                continue;
            }
            let prio = slot.entity.priority;
            if prio > entity.priority {
                continue;
            }
            match candidate {
                None => candidate = Some((idx, prio, slot.entity.age)),
                Some((_, best_prio, best_age)) => {
                    if prio < best_prio || (prio == best_prio && slot.entity.age > best_age) {
                        candidate = Some((idx, prio, slot.entity.age));
                    }
                }
            }
        }
        if let Some((idx, _, _)) = candidate {
            let old_kind = self.slots[idx].entity.kind.index();
            self.active_counts[old_kind] = self.active_counts[old_kind].saturating_sub(1);
            self.slots[idx].entity = entity;
            self.slots[idx].active = true;
            self.active_counts[kind_idx] = self.active_counts[kind_idx].saturating_add(1);
            return Some(idx);
        }
        None
    }

    fn rebuild_free_list(&mut self) {
        self.free_list.clear();
        for (idx, slot) in self.slots.iter().enumerate() {
            if !slot.active {
                self.free_list.push(idx);
            }
        }
    }
}

fn lod_from_zoom(zoom: u16) -> FxLod {
    match zoom {
        0 | 1 => FxLod::Low,
        2 => FxLod::Medium,
        _ => FxLod::High,
    }
}

fn muzzle_profile(kind: TowerKind) -> MuzzleProfile {
    match kind {
        TowerKind::Basic => MuzzleProfile::new("┼", Some("▒"), 2),
        TowerKind::Sniper => MuzzleProfile::new("━", None, 1),
        TowerKind::Rapid => MuzzleProfile::new("▪", Some("·"), 1),
        TowerKind::Cannon => MuzzleProfile::new("◉", Some("▓"), 3),
        TowerKind::Tesla => MuzzleProfile::new("╬", Some("╫"), 2),
        TowerKind::Frost => MuzzleProfile::new("✦", Some("░"), 2),
    }
}

struct MuzzleProfile {
    glyph_main: &'static str,
    glyph_fade: Option<&'static str>,
    ttl: u8,
}

impl MuzzleProfile {
    const fn new(glyph_main: &'static str, glyph_fade: Option<&'static str>, ttl: u8) -> Self {
        Self {
            glyph_main,
            glyph_fade,
            ttl,
        }
    }
}

fn projectile_glyphs(kind: TowerKind) -> (&'static str, &'static str) {
    match kind {
        TowerKind::Basic => ("•", "·"),
        TowerKind::Sniper => ("─", "─"),
        TowerKind::Rapid => ("▪", "·"),
        TowerKind::Cannon => ("●", "◍"),
        TowerKind::Tesla => ("╋", "╌"),
        TowerKind::Frost => ("◆", "░"),
    }
}

fn tower_kind_color(kind: TowerKind) -> Color {
    match kind {
        TowerKind::Basic => Color::White,
        TowerKind::Sniper => Color::Yellow,
        TowerKind::Rapid => Color::Green,
        TowerKind::Cannon => Color::LightRed,
        TowerKind::Tesla => Color::Cyan,
        TowerKind::Frost => Color::Blue,
    }
}

fn line_glyph(from: Vec2i, to: Vec2i) -> &'static str {
    let dx = (to.x - from.x).signum();
    let dy = (to.y - from.y).signum();
    match (dx, dy) {
        (1, 0) | (-1, 0) => "─",
        (0, 1) | (0, -1) => "│",
        (1, 1) | (-1, -1) => "╲",
        (1, -1) | (-1, 1) => "╱",
        _ => "─",
    }
}

fn line_points(from: Vec2i, to: Vec2i) -> Vec<Vec2i> {
    let mut points = Vec::new();
    let mut x0 = from.x;
    let mut y0 = from.y;
    let x1 = to.x;
    let y1 = to.y;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        points.push(Vec2i::new(x0, y0));
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
    points
}

fn draw_screen_cell(
    x: u16,
    y: u16,
    glyph: &'static str,
    color: Color,
    modifier: Modifier,
    buf: &mut Buffer,
    area: Rect,
    budget: &mut i32,
) -> u16 {
    if *budget <= 0 {
        return 0;
    }
    if x < area.x || y < area.y || x >= area.right() || y >= area.bottom() {
        return 0;
    }
    if let Some(cell) = buf.cell_mut((x, y)) {
        let style = cell.style().fg(color).add_modifier(modifier);
        cell.set_symbol(glyph).set_style(style);
        *budget -= 1;
        return 1;
    }
    0
}

fn draw_screen_line(
    from: (u16, u16),
    to: (u16, u16),
    color: Color,
    modifier: Modifier,
    buf: &mut Buffer,
    area: Rect,
    budget: &mut i32,
) -> u16 {
    let (mut x0, mut y0) = (from.0 as i32, from.1 as i32);
    let (x1, y1) = (to.0 as i32, to.1 as i32);

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut used = 0u16;
    let mut prev = (x0, y0);
    loop {
        if *budget <= 0 {
            break;
        }

        let step_dx = (x0 - prev.0).clamp(-1, 1) as i16;
        let step_dy = (y0 - prev.1).clamp(-1, 1) as i16;
        let glyph = if step_dx == 0 && step_dy == 0 {
            // primeiro ponto
            let ddx = (x1 - x0).signum() as i16;
            let ddy = (y1 - y0).signum() as i16;
            line_glyph(Vec2i::default(), Vec2i::new(ddx, ddy))
        } else {
            line_glyph(Vec2i::default(), Vec2i::new(step_dx, step_dy))
        };

        used += draw_screen_cell(
            x0 as u16, y0 as u16, glyph, color, modifier, buf, area, budget,
        );

        if x0 == x1 && y0 == y1 {
            break;
        }

        prev = (x0, y0);
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
    used
}

fn draw_fx_cell(
    pos: Vec2i,
    glyph: &'static str,
    color: Color,
    modifier: Modifier,
    buf: &mut Buffer,
    area: Rect,
    viewport: MapViewport,
    budget: &mut i32,
) -> u16 {
    let Some((cx, cy)) = map_to_screen(pos, area, viewport) else {
        return 0;
    };

    let mut used = 0u16;
    used += draw_screen_cell(cx, cy, glyph, color, modifier, buf, area, budget);

    let tile = viewport.tile_w.min(viewport.tile_h);
    let radius = (tile / 8).min(2);
    if radius == 0 {
        return used;
    }

    let halo = ".";
    let (cx, cy) = (cx as i16, cy as i16);
    let x_min = area.x as i16;
    let y_min = area.y as i16;
    let x_max = area.right().saturating_sub(1) as i16;
    let y_max = area.bottom().saturating_sub(1) as i16;

    const OFFSETS_R1: [(i16, i16); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    const OFFSETS_R2: [(i16, i16); 8] = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (2, 0),
        (-2, 0),
        (0, 2),
        (0, -2),
    ];

    let offsets: &[(i16, i16)] = match radius {
        1 => &OFFSETS_R1,
        2 => &OFFSETS_R2,
        _ => &[],
    };

    for &(dx, dy) in offsets {
        if *budget <= 0 {
            break;
        }
        let x = (cx + dx).clamp(x_min, x_max);
        let y = (cy + dy).clamp(y_min, y_max);
        used += draw_screen_cell(
            x as u16,
            y as u16,
            halo,
            color,
            Modifier::DIM,
            buf,
            area,
            budget,
        );
    }

    used
}

fn draw_cell(
    pos: Vec2i,
    glyph: &'static str,
    color: Color,
    modifier: Modifier,
    buf: &mut Buffer,
    area: Rect,
    viewport: MapViewport,
    budget: &mut i32,
) -> u16 {
    if *budget <= 0 {
        return 0;
    }
    let Some((x, y)) = map_to_screen(pos, area, viewport) else {
        return 0;
    };
    if let Some(cell) = buf.cell_mut((x, y)) {
        let style = cell.style().fg(color).add_modifier(modifier);
        cell.set_symbol(glyph).set_style(style);
        *budget -= 1;
        return 1;
    }
    0
}

fn draw_cardinals(
    center: Vec2i,
    radius: i16,
    glyph: &'static str,
    color: Color,
    buf: &mut Buffer,
    area: Rect,
    viewport: MapViewport,
    budget: &mut i32,
) -> u16 {
    let mut used = 0;
    let offsets = [
        Vec2i::new(radius, 0),
        Vec2i::new(-radius, 0),
        Vec2i::new(0, radius),
        Vec2i::new(0, -radius),
    ];
    for off in offsets {
        if *budget <= 0 {
            break;
        }
        let pos = Vec2i::new(center.x + off.x, center.y + off.y);
        used += draw_cell(
            pos,
            glyph,
            color,
            Modifier::BOLD,
            buf,
            area,
            viewport,
            budget,
        );
    }
    used
}

fn map_to_screen(pos: Vec2i, area: Rect, viewport: MapViewport) -> Option<(u16, u16)> {
    if pos.x < 0 || pos.y < 0 {
        return None;
    }
    let cx = pos.x as u16;
    let cy = pos.y as u16;
    if cx < viewport.view_x || cy < viewport.view_y {
        return None;
    }
    let gx = cx - viewport.view_x;
    let gy = cy - viewport.view_y;
    if gx >= viewport.vis_w || gy >= viewport.vis_h {
        return None;
    }
    let tile_x = area.x + gx * viewport.tile_w;
    let tile_y = area.y + gy * viewport.tile_h;
    let mid_x = tile_x + (viewport.tile_w / 2).min(viewport.tile_w.saturating_sub(1));
    let mid_y = tile_y + (viewport.tile_h / 2).min(viewport.tile_h.saturating_sub(1));
    if mid_x >= area.right() || mid_y >= area.bottom() {
        return None;
    }
    Some((mid_x, mid_y))
}

fn xorshift32(mut x: u32) -> u32 {
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}

fn enemy_kind_color(kind: EnemyKind) -> Color {
    match kind {
        EnemyKind::Swarm => Color::LightRed,
        EnemyKind::Runner => Color::LightYellow,
        EnemyKind::Tank => Color::DarkGray,
        EnemyKind::Shielded => Color::Cyan,
        EnemyKind::Healer => Color::LightGreen,
        EnemyKind::Sneak => Color::Magenta,
    }
}

fn map_to_tile_origin(pos: Vec2i, area: Rect, viewport: MapViewport) -> Option<(u16, u16)> {
    if pos.x < 0 || pos.y < 0 {
        return None;
    }
    let cx = pos.x as u16;
    let cy = pos.y as u16;
    if cx < viewport.view_x || cy < viewport.view_y {
        return None;
    }
    let gx = cx - viewport.view_x;
    let gy = cy - viewport.view_y;
    if gx >= viewport.vis_w || gy >= viewport.vis_h {
        return None;
    }
    let tile_x = area.x + gx * viewport.tile_w;
    let tile_y = area.y + gy * viewport.tile_h;
    if tile_x >= area.right() || tile_y >= area.bottom() {
        return None;
    }
    Some((tile_x, tile_y))
}

/// Render a sprite during death animation.
/// age 0: full sprite dim — corpse flash
/// age 1: ~half chars, scattered `▒`
/// age 2: ~quarter chars, `░`
/// age 3: few `·` dots
fn draw_fx_sprite_death(
    pos: Vec2i,
    age: u8,
    _ttl: u8,
    seed: u32,
    sprite: assets::Sprite,
    color: Color,
    buf: &mut Buffer,
    area: Rect,
    viewport: MapViewport,
    budget: &mut i32,
) -> u16 {
    let Some((tile_x, tile_y)) = map_to_tile_origin(pos, area, viewport) else {
        return 0;
    };
    let tile_w = viewport.tile_w as usize;
    let tile_h = viewport.tile_h as usize;
    let h = sprite.h.min(viewport.tile_h) as usize;
    let w = sprite.w.min(viewport.tile_w) as usize;

    let mut used = 0u16;
    let mut rng = seed;

    // survival probability per cell per age frame
    // age 0 = 100%, age 1 = 60%, age 2 = 30%, age 3 = 10%
    let survive_num = match age {
        0 => 100u32,
        1 => 60,
        2 => 30,
        _ => 10,
    };

    let glyphs: &[&str] = match age {
        0 => &["▓", "▒"],
        1 => &["▒", "░", "·"],
        2 => &["░", "·", " "],
        _ => &["·", " ", " "],
    };

    for sy in 0..h {
        let row = sprite.row(sy);
        for (sx, ch) in row.chars().take(w).enumerate() {
            if *budget <= 0 {
                return used;
            }
            // For age 0 skip spaces (transparent), for later ages also skip
            if ch == ' ' && age == 0 {
                continue;
            }
            rng = xorshift32(rng);
            // probabilistic skip
            if rng % 100 >= survive_num {
                continue;
            }
            let glyph = glyphs[(rng as usize) % glyphs.len()];
            if glyph == " " {
                continue;
            }
            let x = tile_x + ((sx * tile_w) / sprite.w.max(1) as usize) as u16;
            let y = tile_y + ((sy * tile_h) / sprite.h.max(1) as usize) as u16;
            if x >= area.right() || y >= area.bottom() {
                continue;
            }
            let modifier = Modifier::DIM;
            if let Some(cell) = buf.cell_mut((x, y)) {
                let style = ratatui::style::Style::default().fg(color).add_modifier(modifier);
                cell.set_symbol(glyph).set_style(style);
                *budget -= 1;
                used += 1;
            }
        }
    }
    used
}
