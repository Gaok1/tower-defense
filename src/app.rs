use crate::save;
use ratatui::layout::Rect;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    MapSelect,
    LoadGame,
    Game,
}

pub const TOWER_KIND_COUNT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonId {
    StartPause,
    Build,
    Upgrade,
    Sell,
    Speed,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapSelectAction {
    Prev,
    Next,
    Start,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadMenuFocus {
    Slots,
    Waves,
}

#[derive(Debug, Clone)]
pub struct LoadMenuState {
    pub slots: Vec<save::SaveSlotSummary>,
    pub selected_slot: usize,
    pub selected_wave: usize,
    pub focus: LoadMenuFocus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverAction {
    UpgradePreview,
}

#[derive(Debug, Clone, Copy)]
pub struct UiHitboxes {
    pub map_inner: Rect,
    pub buttons: [Rect; 6],
    pub inspector_upgrade: Rect, // linha clicável no inspector
    pub build_options: [Rect; TOWER_KIND_COUNT],
    pub map_select_left: Rect,
    pub map_select_right: Rect,
    pub map_select_start: Rect,
}

impl Default for UiHitboxes {
    fn default() -> Self {
        Self {
            map_inner: Rect::new(0, 0, 0, 0),
            buttons: [Rect::new(0, 0, 0, 0); 6],
            inspector_upgrade: Rect::new(0, 0, 0, 0),
            build_options: [Rect::new(0, 0, 0, 0); TOWER_KIND_COUNT],
            map_select_left: Rect::new(0, 0, 0, 0),
            map_select_right: Rect::new(0, 0, 0, 0),
            map_select_start: Rect::new(0, 0, 0, 0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MapViewport {
    pub tile_w: u16, // colunas por célula (base 2)
    pub tile_h: u16, // linhas por célula (1)
    pub view_x: u16,
    pub view_y: u16,
    pub vis_w: u16,
    pub vis_h: u16,
}

impl Default for MapViewport {
    fn default() -> Self {
        Self {
            tile_w: 2,
            tile_h: 1,
            view_x: 0,
            view_y: 0,
            vis_w: 0,
            vis_h: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub mode: LayoutMode,
    pub hover_button: Option<ButtonId>,
    pub hover_cell: Option<(u16, u16)>,
    pub hover_action: Option<HoverAction>,
    pub hover_build_kind: Option<TowerKind>,
    pub hover_map_select: Option<MapSelectAction>,
    pub hit: UiHitboxes,
    pub viewport: MapViewport,
    pub zoom: u16,
    pub last_zoom: u16, // <-- NOVO (pra ancorar zoom sem “pular”)
    pub manual_pan: bool,
    pub drag_origin: Option<(u16, u16)>,
    pub drag_view: Option<(u16, u16)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TowerKind {
    Basic,
    Sniper,
    Rapid,
    Cannon,
    Tesla,
    Frost,
}

#[derive(Debug, Clone, Copy)]
pub struct Tower {
    pub x: u16,
    pub y: u16,
    pub kind: TowerKind,
    pub level: u8,
    pub cooldown: u16, // ticks até poder atirar
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub path_i: usize,
    pub hp: i32,
    pub move_cd: u16, // ticks até o próximo passo
    pub slow_ticks: u16,
    pub slow_percent: u8,
}

#[derive(Debug, Clone)]
pub struct Projectile {
    pub x: i16,
    pub y: i16,
    pub tx: i16,
    pub ty: i16,
    pub ttl: u16,
    pub damage: i32,
    pub step_cd: u16, // ticks até andar 1 tile (pra dar tempo de ver FX)
    pub kind: TowerKind,
    pub source_level: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum ParticleKind {
    Trail,
    Spark,
    Smoke,
    Arc,
    Shard,
    Bolt,
    Frost,
    Wave,
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub x: i16,
    pub y: i16,
    pub vx: i8,
    pub vy: i8,
    pub ttl: u8,
    pub kind: ParticleKind,
}

#[derive(Debug, Clone)]
pub struct ImpactFx {
    pub x: u16,
    pub y: u16,
    pub ttl: u8,
    pub kind: TowerKind,
}

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub attack: i32,
    pub range: u16,
    pub fire_cd: u16, // ticks entre tiros
}

#[derive(Debug, Clone, Copy)]
pub struct UpgradeDelta {
    pub attack: i32,
    pub range: i16,
    pub fire_cd: i16,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub running: bool,
    pub speed: u8, // 1..=4 (multiplicador de "subtração" de cooldown)

    pub money: i32,
    pub lives: i32,
    pub wave: i32,

    pub grid_w: u16,
    pub grid_h: u16,

    pub selected_cell: Option<(u16, u16)>,
    pub build_kind: Option<TowerKind>,
    pub map_name: String,
    pub pending_build: Option<(u16, u16)>,

    pub path: Vec<(u16, u16)>,
    pub towers: Vec<Tower>,
    pub enemies: Vec<Enemy>,

    pub projectiles: Vec<Projectile>,
    pub impacts: Vec<ImpactFx>,
    pub particles: Vec<Particle>,

    // economia/time
    pub money_cd: u16,
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub screen: Screen,
    pub dev_mode: bool,
    pub save_slot: Option<String>,
    pub last_save_error: Option<String>,
    pub main_menu_index: usize,
    pub load_menu: LoadMenuState,

    tick_rate: Duration,
    last_tick: Instant,

    // RNG simples (sem deps)
    rng: u64,

    pub ui: UiState,
    pub game: GameState,
    maps: Vec<MapSpec>,
    map_index: usize,
}

#[derive(Debug, Clone)]
pub struct MapSpec {
    pub name: &'static str,
    pub grid_w: u16,
    pub grid_h: u16,
    pub path: Vec<(u16, u16)>,
}

impl App {
    pub fn new() -> Self {
        let rng = 0xC0FFEE_u64 ^ (Instant::now().elapsed().as_nanos() as u64);
        let maps = Self::build_maps();
        let map_index = 0usize;
        let map = maps[map_index].clone();
        let selected_cell = Self::first_buildable(map.grid_w, map.grid_h, &map.path);

        let mut app = Self {
            should_quit: false,
            screen: Screen::MainMenu,
            dev_mode: false,
            save_slot: None,
            last_save_error: None,
            main_menu_index: 0,
            load_menu: LoadMenuState {
                slots: vec![],
                selected_slot: 0,
                selected_wave: 0,
                focus: LoadMenuFocus::Slots,
                error: None,
            },
            // tick mais curto -> animações mais suaves
            tick_rate: Duration::from_millis(50),
            last_tick: Instant::now(),
            rng,
            ui: UiState {
                mode: LayoutMode::Wide,
                hover_button: None,
                hover_cell: None,
                hover_action: None,
                hover_build_kind: None,
                hover_map_select: None,
                hit: UiHitboxes::default(),
                viewport: MapViewport::default(),
                zoom: 2,
                last_zoom: 2, // <-- NOVO
                manual_pan: false,
                drag_origin: None,
                drag_view: None,
            },

            game: GameState {
                running: false,
                speed: 1,
                money: 250,
                lives: 20,
                wave: 1,
                grid_w: map.grid_w,
                grid_h: map.grid_h,
                selected_cell: Some(selected_cell),
                build_kind: None,
                map_name: map.name.to_string(),
                pending_build: None,
                path: map.path,
                towers: vec![Tower {
                    x: selected_cell.0,
                    y: selected_cell.1,
                    kind: TowerKind::Basic,
                    level: 1,
                    cooldown: 0,
                }],
                enemies: vec![],
                projectiles: vec![],
                impacts: vec![],
                particles: vec![],
                money_cd: 0,
            },
            maps,
            map_index,
        };

        app.spawn_wave();
        app
    }

    pub fn set_layout_mode_from_size(&mut self, area: Rect) {
        self.ui.mode = if area.width < 96 || area.height < 28 {
            LayoutMode::Compact
        } else {
            LayoutMode::Wide
        };
    }

    pub fn toggle_dev_mode(&mut self) {
        self.dev_mode = !self.dev_mode;
    }

    pub fn main_menu_prev(&mut self) {
        const COUNT: usize = 2; // New game / Load game
        if COUNT == 0 {
            return;
        }
        if self.main_menu_index == 0 {
            self.main_menu_index = COUNT - 1;
        } else {
            self.main_menu_index -= 1;
        }
    }

    pub fn main_menu_next(&mut self) {
        const COUNT: usize = 2; // New game / Load game
        if COUNT == 0 {
            return;
        }
        self.main_menu_index = (self.main_menu_index + 1) % COUNT;
    }

    pub fn main_menu_activate(&mut self) {
        match self.main_menu_index {
            0 => self.screen = Screen::MapSelect,
            1 => self.enter_load_game(),
            _ => {}
        }
    }

    pub fn enter_main_menu(&mut self) {
        self.screen = Screen::MainMenu;
    }

    pub fn enter_load_game(&mut self) {
        self.refresh_load_menu();
        self.screen = Screen::LoadGame;
    }

    pub fn refresh_load_menu(&mut self) {
        self.load_menu.focus = LoadMenuFocus::Slots;
        self.load_menu.selected_slot = 0;
        self.load_menu.selected_wave = 0;

        match save::list_slots() {
            Ok(slots) => {
                self.load_menu.error = None;
                self.load_menu.slots = slots;
            }
            Err(e) => {
                self.load_menu.error = Some(e.to_string());
                self.load_menu.slots = vec![];
            }
        }

        self.clamp_load_menu_selection();
    }

    pub fn load_menu_focus_left(&mut self) {
        self.load_menu.focus = LoadMenuFocus::Slots;
    }

    pub fn load_menu_focus_right(&mut self) {
        self.load_menu.focus = LoadMenuFocus::Waves;
    }

    pub fn load_menu_prev(&mut self) {
        match self.load_menu.focus {
            LoadMenuFocus::Slots => {
                let len = self.load_menu.slots.len();
                if len == 0 {
                    return;
                }
                if self.load_menu.selected_slot == 0 {
                    self.load_menu.selected_slot = len - 1;
                } else {
                    self.load_menu.selected_slot -= 1;
                }
                self.load_menu.selected_wave = self.selected_slot_last_wave_index();
            }
            LoadMenuFocus::Waves => {
                let waves_len = self.selected_slot_waves_len();
                if waves_len == 0 {
                    return;
                }
                if self.load_menu.selected_wave == 0 {
                    self.load_menu.selected_wave = waves_len - 1;
                } else {
                    self.load_menu.selected_wave -= 1;
                }
            }
        }
    }

    pub fn load_menu_next(&mut self) {
        match self.load_menu.focus {
            LoadMenuFocus::Slots => {
                let len = self.load_menu.slots.len();
                if len == 0 {
                    return;
                }
                self.load_menu.selected_slot = (self.load_menu.selected_slot + 1) % len;
                self.load_menu.selected_wave = self.selected_slot_last_wave_index();
            }
            LoadMenuFocus::Waves => {
                let waves_len = self.selected_slot_waves_len();
                if waves_len == 0 {
                    return;
                }
                self.load_menu.selected_wave = (self.load_menu.selected_wave + 1) % waves_len;
            }
        }
    }

    pub fn load_menu_activate(&mut self) {
        let Some(slot) = self.load_menu.slots.get(self.load_menu.selected_slot) else {
            return;
        };
        let Some(&wave) = slot.waves.get(self.load_menu.selected_wave) else {
            return;
        };

        match save::load_checkpoint(&slot.id, wave) {
            Ok(checkpoint) => self.apply_loaded_checkpoint(slot.id.clone(), checkpoint),
            Err(e) => self.load_menu.error = Some(e.to_string()),
        }
    }

    fn clamp_load_menu_selection(&mut self) {
        if self.load_menu.slots.is_empty() {
            self.load_menu.selected_slot = 0;
            self.load_menu.selected_wave = 0;
            return;
        }

        if self.load_menu.selected_slot >= self.load_menu.slots.len() {
            self.load_menu.selected_slot = self.load_menu.slots.len() - 1;
        }
        self.load_menu.selected_wave = self.selected_slot_last_wave_index();
    }

    fn selected_slot_waves_len(&self) -> usize {
        self.load_menu
            .slots
            .get(self.load_menu.selected_slot)
            .map(|s| s.waves.len())
            .unwrap_or(0)
    }

    fn selected_slot_last_wave_index(&self) -> usize {
        self.selected_slot_waves_len().saturating_sub(1)
    }

    fn reset_ui_for_game(&mut self) {
        self.ui.hover_cell = None;
        self.ui.hover_action = None;
        self.ui.hover_build_kind = None;
        self.ui.hover_button = None;
        self.ui.hover_map_select = None;
        self.ui.viewport = MapViewport::default();
        self.ui.manual_pan = false;
        self.ui.drag_origin = None;
        self.ui.drag_view = None;
    }

    fn apply_loaded_checkpoint(&mut self, slot_id: String, checkpoint: save::SaveCheckpoint) {
        let map_index = match self.maps.iter().position(|m| m.name == checkpoint.map_name) {
            Some(i) => i,
            None => {
                self.load_menu.error = Some(format!("map not found: {}", checkpoint.map_name));
                return;
            }
        };

        let map = self.maps[map_index].clone();
        self.map_index = map_index;

        let mut towers: Vec<Tower> = checkpoint
            .towers
            .into_iter()
            .map(|t| Tower {
                x: t.x,
                y: t.y,
                kind: match t.kind {
                    save::TowerKindSave::Basic => TowerKind::Basic,
                    save::TowerKindSave::Sniper => TowerKind::Sniper,
                    save::TowerKindSave::Rapid => TowerKind::Rapid,
                    save::TowerKindSave::Cannon => TowerKind::Cannon,
                    save::TowerKindSave::Tesla => TowerKind::Tesla,
                    save::TowerKindSave::Frost => TowerKind::Frost,
                },
                level: t.level,
                cooldown: 0,
            })
            .collect();

        if towers.is_empty() {
            let selected_cell = Self::first_buildable(map.grid_w, map.grid_h, &map.path);
            towers.push(Tower {
                x: selected_cell.0,
                y: selected_cell.1,
                kind: TowerKind::Basic,
                level: 1,
                cooldown: 0,
            });
        }

        let selected_cell = towers
            .first()
            .map(|t| (t.x, t.y))
            .or_else(|| Some(Self::first_buildable(map.grid_w, map.grid_h, &map.path)));

        self.dev_mode = checkpoint.dev_mode;
        self.save_slot = Some(slot_id);
        self.last_save_error = None;

        self.game = GameState {
            running: false,
            speed: checkpoint.speed.clamp(1, 4),
            money: checkpoint.money,
            lives: checkpoint.lives,
            wave: checkpoint.wave.max(1),
            grid_w: map.grid_w,
            grid_h: map.grid_h,
            selected_cell,
            build_kind: None,
            map_name: map.name.to_string(),
            pending_build: None,
            path: map.path,
            towers,
            enemies: vec![],
            projectiles: vec![],
            impacts: vec![],
            particles: vec![],
            money_cd: 0,
        };

        self.reset_ui_for_game();
        self.screen = Screen::Game;
        self.spawn_wave();
    }

    pub fn on_tick_if_due(&mut self) {
        if self.last_tick.elapsed() >= self.tick_rate {
            self.last_tick = Instant::now();
            if self.screen == Screen::Game {
                self.on_tick();
            } else {
                self.tick_fx();
            }
        }
    }

    pub fn handle_button(&mut self, id: ButtonId) {
        match id {
            ButtonId::StartPause => self.game.running = !self.game.running,
            ButtonId::Build => self.try_build(),
            ButtonId::Upgrade => self.try_upgrade(),
            ButtonId::Sell => self.try_sell(),
            ButtonId::Speed => self.cycle_speed(),
            ButtonId::Quit => self.should_quit = true,
        }
    }

    fn on_tick(&mut self) {
        // VFX rodam mesmo pausado (UI continua viva)
        self.tick_fx();

        if !self.game.running {
            return;
        }

        self.tick_enemies();
        self.tick_towers();
        self.tick_projectiles();
        self.tick_economy_and_waves();

        if !self.dev_mode && self.game.lives <= 0 {
            self.game.running = false;
        }
    }

    fn tick_fx(&mut self) {
        for fx in &mut self.game.impacts {
            fx.ttl = fx.ttl.saturating_sub(1);
        }
        self.game.impacts.retain(|fx| fx.ttl > 0);

        for p in &mut self.game.particles {
            // partículas "andam" lentamente em grid
            if p.vx != 0 {
                p.x += p.vx.signum() as i16;
                p.vx -= p.vx.signum();
            }
            if p.vy != 0 {
                p.y += p.vy.signum() as i16;
                p.vy -= p.vy.signum();
            }
            p.ttl = p.ttl.saturating_sub(1);
        }
        self.game.particles.retain(|p| p.ttl > 0);
    }

    fn tick_enemies(&mut self) {
        let sp = self.game.speed.max(1) as u16;
        let base = self.enemy_base_move_cd();

        for e in &mut self.game.enemies {
            if e.hp <= 0 {
                continue;
            }

            if e.slow_ticks > 0 {
                e.slow_ticks = e.slow_ticks.saturating_sub(1);
            } else {
                e.slow_percent = 0;
            }
            let slow_factor = 100u16.saturating_sub(e.slow_percent as u16);
            let effective_sp = (sp * slow_factor / 100).max(1);

            if e.move_cd > effective_sp {
                e.move_cd -= effective_sp;
                continue;
            }

            // move 1 tile
            e.move_cd = base;
            if e.path_i + 1 < self.game.path.len() {
                e.path_i += 1;
            } else {
                e.hp = 0;
                if !self.dev_mode {
                    self.game.lives = self.game.lives.saturating_sub(1);
                }
            }
        }
    }

    fn tick_towers(&mut self) {
        let sp = self.game.speed.max(1) as u16;
        let mut spawns: Vec<(u16, u16, u16, u16, i32, TowerKind, u8)> = Vec::new();

        for t in &mut self.game.towers {
            if t.cooldown > sp {
                t.cooldown -= sp;
                continue;
            }
            t.cooldown = 0;

            let stats = Self::tower_stats(t);
            let Some((tx, ty)) = Self::acquire_target(
                self.game.enemies.as_slice(),
                self.game.path.as_slice(),
                t.x,
                t.y,
                stats.range,
            ) else {
                continue;
            };

            spawns.push((t.x, t.y, tx, ty, stats.attack, t.kind, t.level));
            t.cooldown = stats.fire_cd;
        }

        for (from_x, from_y, to_x, to_y, dmg, kind, level) in spawns {
            self.spawn_projectile(from_x, from_y, to_x, to_y, dmg, kind, level);
        }
    }

    fn tick_projectiles(&mut self) {
        let sp = self.game.speed.max(1) as u16;
        let mut trails: Vec<(i16, i16)> = Vec::new();
        let mut impacts: Vec<(u16, u16, TowerKind, u8)> = Vec::new();

        for p in &mut self.game.projectiles {
            if p.ttl > 0 {
                p.ttl -= 1;
            }
            if p.ttl == 0 {
                continue;
            }

            if p.step_cd > sp {
                p.step_cd -= sp;
                continue;
            }
            p.step_cd = Self::projectile_step_cd(p.kind);

            let old_x = p.x;
            let old_y = p.y;

            // move 1 passo em direção ao target (grid)
            let dx = (p.tx - p.x).signum();
            let dy = (p.ty - p.y).signum();
            p.x += dx;
            p.y += dy;

            // trail
            trails.push((old_x, old_y));

            if p.x == p.tx && p.y == p.ty {
                let hit_x = p.x.max(0) as u16;
                let hit_y = p.y.max(0) as u16;

                if let Some(ei) = Self::enemy_index_at(
                    self.game.enemies.as_slice(),
                    self.game.path.as_slice(),
                    hit_x,
                    hit_y,
                ) {
                    let e = &mut self.game.enemies[ei];
                    e.hp -= p.damage;
                    if e.hp < 0 {
                        e.hp = 0;
                    }
                    if p.kind == TowerKind::Frost {
                        let (slow_percent, slow_ticks) = Self::frost_slow(p.source_level);
                        e.slow_percent = e.slow_percent.max(slow_percent);
                        e.slow_ticks = e.slow_ticks.max(slow_ticks);
                    }
                }
                if p.kind == TowerKind::Tesla {
                    self.apply_tesla_chain(hit_x, hit_y, p.damage, p.source_level);
                }
                if p.kind == TowerKind::Cannon {
                    self.apply_cannon_splash(hit_x, hit_y, p.damage, p.source_level);
                }
                if p.kind == TowerKind::Frost {
                    self.apply_frost_burst(hit_x, hit_y, p.source_level);
                }

                impacts.push((hit_x, hit_y, p.kind, p.source_level));
                p.ttl = 0;
            }
        }

        self.game.projectiles.retain(|p| p.ttl > 0);

        for (x, y) in trails {
            self.spawn_trail(x, y);
        }
        for (x, y, kind, level) in impacts {
            self.spawn_impact(x, y, kind, level);
        }
    }

    fn tick_economy_and_waves(&mut self) {
        if self.dev_mode {
            self.game.money_cd = 1;
        }
        // dinheiro por segundo (não por tick)
        if self.game.money_cd > 0 {
            self.game.money_cd -= 1;
        } else {
            // ganho lento, pra combinar com ritmo mais "tático"
            self.game.money = self.game.money.saturating_add(2);
            self.game.money_cd = 20; // 20 ticks * 50ms = 1s
        }

        let alive = self.game.enemies.iter().any(|e| e.hp > 0);
        if !alive && (self.dev_mode || self.game.lives > 0) {
            self.game.wave += 1;
            self.spawn_wave();
        }
    }

    fn spawn_wave(&mut self) {
        // wave com mais inimigos, mas ritmo mais lento.
        let count = (2 + (self.game.wave / 2)).clamp(2, 10) as usize;
        let hp = 45 + self.game.wave * 10;
        let base = self.enemy_base_move_cd();

        self.game.enemies.clear();
        for i in 0..count {
            // pequena defasagem no spawn pelo move_cd inicial
            let stagger = (i as u16 * 6).min(60);
            self.game.enemies.push(Enemy {
                path_i: 0,
                hp,
                move_cd: base + stagger,
                slow_ticks: 0,
                slow_percent: 0,
            });
        }

        // limpa FX antigos pra não virar bagunça visual ao trocar wave
        self.game.projectiles.clear();
        self.game.impacts.clear();
        self.game.particles.clear();

        self.autosave_wave();
    }

    fn autosave_wave(&mut self) {
        let Some(slot_id) = self.save_slot.clone() else {
            return;
        };

        let saved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let towers = self
            .game
            .towers
            .iter()
            .map(|t| save::SaveTower {
                x: t.x,
                y: t.y,
                kind: match t.kind {
                    TowerKind::Basic => save::TowerKindSave::Basic,
                    TowerKind::Sniper => save::TowerKindSave::Sniper,
                    TowerKind::Rapid => save::TowerKindSave::Rapid,
                    TowerKind::Cannon => save::TowerKindSave::Cannon,
                    TowerKind::Tesla => save::TowerKindSave::Tesla,
                    TowerKind::Frost => save::TowerKindSave::Frost,
                },
                level: t.level,
            })
            .collect();

        let checkpoint = save::SaveCheckpoint {
            version: save::SAVE_VERSION,
            saved_at,
            map_name: self.game.map_name.clone(),
            dev_mode: self.dev_mode,
            wave: self.game.wave,
            money: self.game.money,
            lives: self.game.lives,
            speed: self.game.speed,
            towers,
        };

        match save::write_wave_checkpoint(&slot_id, &checkpoint) {
            Ok(()) => self.last_save_error = None,
            Err(e) => self.last_save_error = Some(e.to_string()),
        }
    }

    fn enemy_base_move_cd(&self) -> u16 {
        // 50ms por tick
        // 14 ticks = 700ms por tile (bem mais lento)
        14
    }

    fn projectile_step_cd(kind: TowerKind) -> u16 {
        match kind {
            TowerKind::Rapid => 1,
            TowerKind::Tesla => 1,
            TowerKind::Basic => 2,
            TowerKind::Frost => 2,
            TowerKind::Cannon => 3,
            TowerKind::Sniper => 3,
        }
    }

    fn spawn_projectile(
        &mut self,
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
        dmg: i32,
        kind: TowerKind,
        level: u8,
    ) {
        let ttl = match kind {
            TowerKind::Sniper => 120,
            TowerKind::Cannon => 110,
            TowerKind::Tesla => 80,
            TowerKind::Frost => 90,
            TowerKind::Rapid => 70,
            TowerKind::Basic => 90,
        };

        self.game.projectiles.push(Projectile {
            x: from_x as i16,
            y: from_y as i16,
            tx: to_x as i16,
            ty: to_y as i16,
            ttl,
            damage: dmg,
            step_cd: Self::projectile_step_cd(kind),
            kind,
            source_level: level,
        });

        // pequeno flash de "muzzle" na torre
        self.spawn_spark(from_x as i16, from_y as i16);
        match kind {
            TowerKind::Tesla => {
                let count = 1 + (level / 2).max(1);
                for _ in 0..count {
                    self.spawn_arc(from_x as i16, from_y as i16);
                }
            }
            TowerKind::Cannon => {
                for _ in 0..2 {
                    self.spawn_smoke(from_x as i16, from_y as i16);
                }
                for _ in 0..(1 + level / 2) {
                    self.spawn_wave(from_x as i16, from_y as i16);
                }
            }
            TowerKind::Frost => {
                for _ in 0..(2 + level / 3) {
                    self.spawn_shard(from_x as i16, from_y as i16);
                }
                self.spawn_frost(from_x as i16, from_y as i16);
            }
            _ => {}
        }
    }

    fn spawn_trail(&mut self, x: i16, y: i16) {
        self.game.particles.push(Particle {
            x,
            y,
            vx: 0,
            vy: 0,
            ttl: 4,
            kind: ParticleKind::Trail,
        });
    }

    fn spawn_spark(&mut self, x: i16, y: i16) {
        // fagulhas curtinhas (pequeno espalhamento)
        for _ in 0..3 {
            let vx = self.rand_i8(-1, 1);
            let vy = self.rand_i8(-1, 1);
            self.game.particles.push(Particle {
                x,
                y,
                vx,
                vy,
                ttl: 5,
                kind: ParticleKind::Spark,
            });
        }
    }

    fn spawn_impact(&mut self, x: u16, y: u16, kind: TowerKind, level: u8) {
        self.game.impacts.push(ImpactFx { x, y, ttl: 4, kind });

        let ix = x as i16;
        let iy = y as i16;

        // fagulhas
        for _ in 0..(6 + level as usize) {
            let vx = self.rand_i8(-2, 2);
            let vy = self.rand_i8(-2, 2);
            self.game.particles.push(Particle {
                x: ix,
                y: iy,
                vx,
                vy,
                ttl: 6,
                kind: ParticleKind::Spark,
            });
        }

        // "fumacinha" no ponto de impacto
        self.spawn_smoke(ix, iy);
        if kind == TowerKind::Tesla {
            for _ in 0..(4 + level / 2) {
                self.spawn_arc(ix, iy);
            }
            for _ in 0..(2 + level / 3) {
                self.spawn_bolt(ix, iy);
            }
        }
        if kind == TowerKind::Frost {
            for _ in 0..(4 + level / 2) {
                self.spawn_shard(ix, iy);
            }
            for _ in 0..(2 + level / 3) {
                self.spawn_frost(ix, iy);
            }
        }
        if kind == TowerKind::Cannon {
            for _ in 0..(2 + level / 2) {
                self.spawn_smoke(ix, iy);
            }
            for _ in 0..(2 + level / 3) {
                self.spawn_wave(ix, iy);
            }
        }
    }

    fn spawn_arc(&mut self, x: i16, y: i16) {
        let vx = self.rand_i8(-2, 2);
        let vy = self.rand_i8(-1, 1);
        self.game.particles.push(Particle {
            x,
            y,
            vx,
            vy,
            ttl: 5,
            kind: ParticleKind::Arc,
        });
    }

    fn spawn_shard(&mut self, x: i16, y: i16) {
        let vx = self.rand_i8(-1, 1);
        let vy = self.rand_i8(-2, 0);
        self.game.particles.push(Particle {
            x,
            y,
            vx,
            vy,
            ttl: 6,
            kind: ParticleKind::Shard,
        });
    }

    fn spawn_bolt(&mut self, x: i16, y: i16) {
        self.game.particles.push(Particle {
            x,
            y,
            vx: 0,
            vy: 0,
            ttl: 4,
            kind: ParticleKind::Bolt,
        });
    }

    fn spawn_frost(&mut self, x: i16, y: i16) {
        self.game.particles.push(Particle {
            x,
            y,
            vx: 0,
            vy: 0,
            ttl: 5,
            kind: ParticleKind::Frost,
        });
    }

    fn spawn_wave(&mut self, x: i16, y: i16) {
        self.game.particles.push(Particle {
            x,
            y,
            vx: 0,
            vy: 0,
            ttl: 4,
            kind: ParticleKind::Wave,
        });
    }

    fn spawn_smoke(&mut self, x: i16, y: i16) {
        self.game.particles.push(Particle {
            x,
            y,
            vx: 0,
            vy: 0,
            ttl: 10,
            kind: ParticleKind::Smoke,
        });
    }

    fn spawn_bolt_line(&mut self, from_x: i16, from_y: i16, to_x: i16, to_y: i16) {
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        let steps = dx.abs().max(dy.abs());
        if steps == 0 {
            return;
        }
        let step_x = dx.signum();
        let step_y = dy.signum();
        let mut cx = from_x;
        let mut cy = from_y;
        for _ in 0..steps {
            cx += step_x;
            cy += step_y;
            self.spawn_bolt(cx, cy);
        }
    }

    fn apply_tesla_chain(&mut self, x: u16, y: u16, damage: i32, level: u8) {
        let (radius, max_targets, percent) = Self::tesla_chain_params(level);
        let mut candidates: Vec<(usize, u16, u16, u16)> = Vec::new();
        for (idx, e) in self.game.enemies.iter().enumerate() {
            if e.hp <= 0 {
                continue;
            }
            let (ex, ey) = self.game.path[e.path_i];
            let dist = manhattan(x, y, ex, ey);
            if dist == 0 || dist > radius {
                continue;
            }
            candidates.push((idx, ex, ey, dist));
        }
        candidates.sort_by_key(|&(_, _, _, dist)| dist);

        for (idx, ex, ey, dist) in candidates.into_iter().take(max_targets) {
            let falloff = 1.0 - ((dist - 1) as f32 * 0.18).clamp(0.0, 0.6);
            let chain_damage =
                ((damage as f32) * (percent as f32 / 100.0) * falloff).round() as i32;
            if chain_damage <= 0 {
                continue;
            }
            if let Some(e) = self.game.enemies.get_mut(idx) {
                e.hp -= chain_damage;
                if e.hp < 0 {
                    e.hp = 0;
                }
            }
            self.spawn_bolt_line(x as i16, y as i16, ex as i16, ey as i16);
            self.spawn_arc(ex as i16, ey as i16);
        }
    }

    fn apply_cannon_splash(&mut self, x: u16, y: u16, damage: i32, level: u8) {
        let (radius, percent) = Self::cannon_splash_params(level);
        for e in &mut self.game.enemies {
            if e.hp <= 0 {
                continue;
            }
            let (ex, ey) = self.game.path[e.path_i];
            let dist = manhattan(x, y, ex, ey);
            if dist == 0 || dist > radius {
                continue;
            }
            let splash_damage = ((damage as f32) * (percent as f32 / 100.0)).round() as i32;
            if splash_damage <= 0 {
                continue;
            }
            e.hp -= splash_damage;
            if e.hp < 0 {
                e.hp = 0;
            }
            self.spawn_wave(ex as i16, ey as i16);
            self.spawn_smoke(ex as i16, ey as i16);
        }
    }

    fn apply_frost_burst(&mut self, x: u16, y: u16, level: u8) {
        let (radius, slow_percent, slow_ticks) = Self::frost_burst_params(level);
        for e in &mut self.game.enemies {
            if e.hp <= 0 {
                continue;
            }
            let (ex, ey) = self.game.path[e.path_i];
            let dist = manhattan(x, y, ex, ey);
            if dist == 0 || dist > radius {
                continue;
            }
            e.slow_percent = e.slow_percent.max(slow_percent);
            e.slow_ticks = e.slow_ticks.max(slow_ticks);
            self.spawn_frost(ex as i16, ey as i16);
        }
    }

    fn rand_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D_u64)) >> 32) as u32
    }

    fn rand_i8(&mut self, lo: i8, hi: i8) -> i8 {
        if lo >= hi {
            return lo;
        }
        let span = (hi as i16 - lo as i16 + 1) as u16;
        let v = (self.rand_u32() % span as u32) as i16;
        (lo as i16 + v) as i8
    }

    fn acquire_target(
        enemies: &[Enemy],
        path: &[(u16, u16)],
        x: u16,
        y: u16,
        range: u16,
    ) -> Option<(u16, u16)> {
        let mut best: Option<(u16, u16, u16)> = None; // (ex, ey, dist)
        for e in enemies {
            if e.hp <= 0 {
                continue;
            }
            let (ex, ey) = path[e.path_i];
            let dist = manhattan(x, y, ex, ey);
            if dist <= range {
                match best {
                    None => best = Some((ex, ey, dist)),
                    Some((_, _, bd)) if dist < bd => best = Some((ex, ey, dist)),
                    _ => {}
                }
            }
        }
        best.map(|(ex, ey, _)| (ex, ey))
    }

    pub fn is_path(&self, x: u16, y: u16) -> bool {
        self.game.path.iter().any(|&(px, py)| px == x && py == y)
    }

    pub fn enemy_at(&self, x: u16, y: u16) -> bool {
        Self::enemy_index_at(
            self.game.enemies.as_slice(),
            self.game.path.as_slice(),
            x,
            y,
        )
        .is_some()
    }

    fn enemy_index_at(enemies: &[Enemy], path: &[(u16, u16)], x: u16, y: u16) -> Option<usize> {
        for (i, e) in enemies.iter().enumerate() {
            if e.hp <= 0 {
                continue;
            }
            let (ex, ey) = path[e.path_i];
            if ex == x && ey == y {
                return Some(i);
            }
        }
        None
    }

    pub fn tower_index_at(&self, x: u16, y: u16) -> Option<usize> {
        self.game.towers.iter().position(|t| t.x == x && t.y == y)
    }

    pub fn selected_tower(&self) -> Option<&Tower> {
        let (x, y) = self.game.selected_cell?;
        let idx = self.tower_index_at(x, y)?;
        self.game.towers.get(idx)
    }

    pub fn tower_stats(t: &Tower) -> Stats {
        let tuning = Self::tower_tuning(t.kind);
        let lvl = t.level.max(1) as i32;
        let attack = tuning.base_attack + (lvl - 1) * tuning.attack_step;
        let lvl_steps = u16::from(t.level.saturating_sub(1));
        let range = tuning.base_range + (lvl_steps / tuning.range_every);
        let cd_drop = ((lvl_steps / tuning.cd_drop_every) as i32) * tuning.cd_drop;
        let fire_cd = (tuning.base_cd - cd_drop).clamp(tuning.cd_min, tuning.cd_max) as u16;

        Stats {
            attack,
            range,
            fire_cd,
        }
    }

    pub fn upgrade_delta(&self, t: &Tower) -> UpgradeDelta {
        let next_level = (t.level + 1).min(9);
        if next_level == t.level {
            return UpgradeDelta {
                attack: 0,
                range: 0,
                fire_cd: 0,
            };
        }

        let now = Self::tower_stats(t);
        let mut t2 = *t;
        t2.level = next_level;
        let nxt = Self::tower_stats(&t2);

        UpgradeDelta {
            attack: nxt.attack - now.attack,
            range: nxt.range as i16 - now.range as i16,
            fire_cd: nxt.fire_cd as i16 - now.fire_cd as i16,
        }
    }

    fn cycle_speed(&mut self) {
        self.game.speed = match self.game.speed {
            1 => 2,
            2 => 3,
            3 => 4,
            _ => 1,
        };
        // tick_rate fica fixo pra manter animações consistentes
    }

    pub fn cycle_zoom(&mut self, delta: i16) {
        let next = (self.ui.zoom as i16 + delta).clamp(1, 4) as u16;
        self.ui.zoom = next;
    }

    pub fn maps_len(&self) -> usize {
        self.maps.len()
    }

    pub fn selected_map_index(&self) -> usize {
        self.map_index
    }

    pub fn selected_map(&self) -> &MapSpec {
        &self.maps[self.map_index]
    }

    pub fn select_prev_map(&mut self) {
        if self.maps.is_empty() {
            return;
        }
        if self.map_index == 0 {
            self.map_index = self.maps.len() - 1;
        } else {
            self.map_index -= 1;
        }
    }

    pub fn select_next_map(&mut self) {
        if self.maps.is_empty() {
            return;
        }
        self.map_index = (self.map_index + 1) % self.maps.len();
    }

    pub fn start_selected_map(&mut self) {
        let map = self.selected_map().clone();
        let selected_cell = Self::first_buildable(map.grid_w, map.grid_h, &map.path);

        self.game = GameState {
            running: false,
            speed: 1,
            money: 250,
            lives: 20,
            wave: 1,
            grid_w: map.grid_w,
            grid_h: map.grid_h,
            selected_cell: Some(selected_cell),
            build_kind: None,
            map_name: map.name.to_string(),
            pending_build: None,
            path: map.path,
            towers: vec![Tower {
                x: selected_cell.0,
                y: selected_cell.1,
                kind: TowerKind::Basic,
                level: 1,
                cooldown: 0,
            }],
            enemies: vec![],
            projectiles: vec![],
            impacts: vec![],
            particles: vec![],
            money_cd: 0,
        };

        self.reset_ui_for_game();
        self.screen = Screen::Game;

        match save::create_new_slot(&self.game.map_name, self.dev_mode) {
            Ok(id) => {
                self.save_slot = Some(id);
                self.last_save_error = None;
            }
            Err(e) => {
                self.save_slot = None;
                self.last_save_error = Some(e.to_string());
            }
        }

        self.spawn_wave();
    }

    fn try_build(&mut self) {
        let Some(kind) = self.game.build_kind else {
            return;
        };
        let Some((x, y)) = self.game.selected_cell else {
            return;
        };
        self.game.pending_build = None;
        if self.build_at(x, y, kind) {
            self.game.build_kind = None;
            self.game.selected_cell = Some((x, y));
        }
    }

    pub fn can_build_at(&self, x: u16, y: u16, kind: TowerKind) -> bool {
        if self.is_path(x, y) {
            return false;
        }
        if self.tower_index_at(x, y).is_some() {
            return false;
        }
        self.dev_mode || self.game.money >= Self::tower_cost(kind)
    }

    pub fn build_at(&mut self, x: u16, y: u16, kind: TowerKind) -> bool {
        if !self.can_build_at(x, y, kind) {
            return false;
        }
        if !self.dev_mode {
            let cost = Self::tower_cost(kind);
            self.game.money -= cost;
        }
        self.game.towers.push(Tower {
            x,
            y,
            kind,
            level: 1,
            cooldown: 0,
        });
        true
    }

    fn try_upgrade(&mut self) {
        let Some((x, y)) = self.game.selected_cell else {
            return;
        };
        let Some(idx) = self.tower_index_at(x, y) else {
            return;
        };

        let cost = Self::tower_upgrade_cost(self.game.towers[idx].kind);
        if !self.dev_mode && self.game.money < cost {
            return;
        }
        let t = &mut self.game.towers[idx];
        if t.level >= 9 {
            return;
        }

        if !self.dev_mode {
            self.game.money -= cost;
        }
        t.level += 1;
    }

    fn try_sell(&mut self) {
        let Some((x, y)) = self.game.selected_cell else {
            return;
        };
        let Some(idx) = self.tower_index_at(x, y) else {
            return;
        };
        self.game.towers.remove(idx);
        if !self.dev_mode {
            self.game.money = self.game.money.saturating_add(20);
        }
    }

    pub fn wave_progress_percent(&self) -> u16 {
        let path_len = self.game.path.len().max(1) as i32;
        let mut best = 0i32;
        for e in &self.game.enemies {
            if e.hp <= 0 {
                continue;
            }
            best = best.max(e.path_i as i32);
        }
        ((best * 100) / (path_len - 1).max(1)).clamp(0, 100) as u16
    }
}

fn manhattan(x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
    x1.abs_diff(x2) + y1.abs_diff(y2)
}

#[derive(Debug, Clone, Copy)]
struct TowerTuning {
    base_attack: i32,
    attack_step: i32,
    base_range: u16,
    range_every: u16,
    base_cd: i32,
    cd_drop_every: u16,
    cd_drop: i32,
    cd_min: i32,
    cd_max: i32,
}

impl App {
    fn tower_tuning(kind: TowerKind) -> TowerTuning {
        match kind {
            TowerKind::Basic => TowerTuning {
                base_attack: 40,
                attack_step: 20,
                base_range: 6,
                range_every: 2,
                base_cd: 18,
                cd_drop_every: 3,
                cd_drop: 2,
                cd_min: 8,
                cd_max: 24,
            },
            TowerKind::Sniper => TowerTuning {
                base_attack: 90,
                attack_step: 30,
                base_range: 9,
                range_every: 3,
                base_cd: 26,
                cd_drop_every: 4,
                cd_drop: 2,
                cd_min: 14,
                cd_max: 30,
            },
            TowerKind::Rapid => TowerTuning {
                base_attack: 22,
                attack_step: 8,
                base_range: 5,
                range_every: 4,
                base_cd: 12,
                cd_drop_every: 2,
                cd_drop: 1,
                cd_min: 6,
                cd_max: 18,
            },
            TowerKind::Cannon => TowerTuning {
                base_attack: 120,
                attack_step: 40,
                base_range: 6,
                range_every: 3,
                base_cd: 32,
                cd_drop_every: 3,
                cd_drop: 2,
                cd_min: 16,
                cd_max: 36,
            },
            TowerKind::Tesla => TowerTuning {
                base_attack: 55,
                attack_step: 18,
                base_range: 7,
                range_every: 2,
                base_cd: 14,
                cd_drop_every: 3,
                cd_drop: 1,
                cd_min: 8,
                cd_max: 18,
            },
            TowerKind::Frost => TowerTuning {
                base_attack: 30,
                attack_step: 12,
                base_range: 7,
                range_every: 2,
                base_cd: 16,
                cd_drop_every: 2,
                cd_drop: 1,
                cd_min: 10,
                cd_max: 20,
            },
        }
    }

    pub fn tower_cost(kind: TowerKind) -> i32 {
        match kind {
            TowerKind::Basic => 50,
            TowerKind::Sniper => 80,
            TowerKind::Rapid => 45,
            TowerKind::Cannon => 95,
            TowerKind::Tesla => 70,
            TowerKind::Frost => 60,
        }
    }

    pub fn tower_upgrade_cost(kind: TowerKind) -> i32 {
        match kind {
            TowerKind::Basic => 30,
            TowerKind::Sniper => 40,
            TowerKind::Rapid => 25,
            TowerKind::Cannon => 45,
            TowerKind::Tesla => 35,
            TowerKind::Frost => 30,
        }
    }

    pub fn build_preview_stats(&self) -> Option<Stats> {
        let kind = self.game.build_kind?;
        let t = Tower {
            x: 0,
            y: 0,
            kind,
            level: 1,
            cooldown: 0,
        };
        Some(Self::tower_stats(&t))
    }

    pub fn available_towers() -> [TowerKind; TOWER_KIND_COUNT] {
        [
            TowerKind::Basic,
            TowerKind::Sniper,
            TowerKind::Rapid,
            TowerKind::Cannon,
            TowerKind::Tesla,
            TowerKind::Frost,
        ]
    }

    pub fn toggle_build_kind(&mut self, kind: TowerKind) {
        self.game.build_kind = match self.game.build_kind {
            Some(current) if current == kind => None,
            _ => Some(kind),
        };
        self.game.pending_build = None;
    }

    pub fn frost_slow(level: u8) -> (u8, u16) {
        let lvl = level.max(1) as u16;
        let slow_percent = (20 + lvl * 3).min(60) as u8;
        let slow_ticks = 8 + lvl * 2;
        (slow_percent, slow_ticks)
    }

    pub fn frost_burst_params(level: u8) -> (u16, u8, u16) {
        let (slow_percent, slow_ticks) = Self::frost_slow(level);
        let radius = if level >= 4 { 2 } else { 1 };
        (radius, (slow_percent / 2).max(10), (slow_ticks / 2).max(12))
    }

    pub fn tesla_chain_params(level: u8) -> (u16, usize, u8) {
        let l = level.max(1);
        let radius = if l >= 6 {
            3
        } else if l >= 3 {
            2
        } else {
            1
        };
        let max_targets = (1 + l / 2).max(2) as usize;
        let percent = (35 + l * 4).min(70);
        (radius, max_targets, percent)
    }

    pub fn cannon_splash_params(level: u8) -> (u16, u8) {
        let l = level.max(1);
        let radius = if l >= 5 { 2 } else { 1 };
        let percent = (30 + l * 3).min(55);
        (radius, percent)
    }

    fn build_maps() -> Vec<MapSpec> {
        vec![
            Self::map_serpentine(),
            Self::map_cascade(),
            Self::map_spiral(),
            Self::map_switchback(),
            Self::map_crosswind(),
        ]
    }

    fn map_serpentine() -> MapSpec {
        let grid_w = 36;
        let grid_h = 18;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 3), (grid_w - 2, 3));
        Self::push_segment(&mut path, (grid_w - 2, 3), (grid_w - 2, grid_h - 3));
        Self::push_segment(&mut path, (grid_w - 2, grid_h - 3), (2, grid_h - 3));
        MapSpec {
            name: "Serpentine",
            grid_w,
            grid_h,
            path,
        }
    }

    fn map_cascade() -> MapSpec {
        let grid_w = 44;
        let grid_h = 22;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 2), (grid_w - 3, 2));
        Self::push_segment(&mut path, (grid_w - 3, 2), (grid_w - 3, 10));
        Self::push_segment(&mut path, (grid_w - 3, 10), (3, 10));
        Self::push_segment(&mut path, (3, 10), (3, grid_h - 3));
        Self::push_segment(&mut path, (3, grid_h - 3), (grid_w - 2, grid_h - 3));
        MapSpec {
            name: "Cascade",
            grid_w,
            grid_h,
            path,
        }
    }

    fn map_spiral() -> MapSpec {
        let grid_w = 50;
        let grid_h = 24;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 3), (grid_w - 2, 3));
        Self::push_segment(&mut path, (grid_w - 2, 3), (grid_w - 2, grid_h - 4));
        Self::push_segment(&mut path, (grid_w - 2, grid_h - 4), (3, grid_h - 4));
        Self::push_segment(&mut path, (3, grid_h - 4), (3, 6));
        Self::push_segment(&mut path, (3, 6), (grid_w - 4, 6));
        Self::push_segment(&mut path, (grid_w - 4, 6), (grid_w - 4, grid_h - 6));
        Self::push_segment(&mut path, (grid_w - 4, grid_h - 6), (6, grid_h - 6));
        MapSpec {
            name: "Spiral",
            grid_w,
            grid_h,
            path,
        }
    }

    fn map_switchback() -> MapSpec {
        let grid_w = 46;
        let grid_h = 20;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 4), (grid_w - 2, 4));
        Self::push_segment(&mut path, (grid_w - 2, 4), (grid_w - 2, 8));
        Self::push_segment(&mut path, (grid_w - 2, 8), (2, 8));
        Self::push_segment(&mut path, (2, 8), (2, 12));
        Self::push_segment(&mut path, (2, 12), (grid_w - 3, 12));
        Self::push_segment(&mut path, (grid_w - 3, 12), (grid_w - 3, grid_h - 3));
        Self::push_segment(&mut path, (grid_w - 3, grid_h - 3), (6, grid_h - 3));
        MapSpec {
            name: "Switchback",
            grid_w,
            grid_h,
            path,
        }
    }

    fn map_crosswind() -> MapSpec {
        let grid_w = 48;
        let grid_h = 22;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 3), (grid_w - 2, 3));
        Self::push_segment(&mut path, (grid_w - 2, 3), (grid_w - 2, 7));
        Self::push_segment(&mut path, (grid_w - 2, 7), (4, 7));
        Self::push_segment(&mut path, (4, 7), (4, 15));
        Self::push_segment(&mut path, (4, 15), (grid_w - 4, 15));
        Self::push_segment(&mut path, (grid_w - 4, 15), (grid_w - 4, grid_h - 3));
        Self::push_segment(&mut path, (grid_w - 4, grid_h - 3), (2, grid_h - 3));
        MapSpec {
            name: "Crosswind",
            grid_w,
            grid_h,
            path,
        }
    }

    fn push_segment(path: &mut Vec<(u16, u16)>, from: (u16, u16), to: (u16, u16)) {
        let (x1, y1) = from;
        let (x2, y2) = to;
        if x1 == x2 {
            let (start, end) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
            for y in start..=end {
                path.push((x1, y));
            }
        } else if y1 == y2 {
            let (start, end) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
            for x in start..=end {
                path.push((x, y1));
            }
        }
    }

    fn first_buildable(grid_w: u16, grid_h: u16, path: &[(u16, u16)]) -> (u16, u16) {
        for y in 1..grid_h.saturating_sub(1) {
            for x in 1..grid_w.saturating_sub(1) {
                if !path.iter().any(|&(px, py)| px == x && py == y) {
                    return (x, y);
                }
            }
        }
        (1, 1)
    }
}
