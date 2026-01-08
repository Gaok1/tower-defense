use ratatui::layout::Rect;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    Compact,
}

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
pub enum HoverAction {
    UpgradePreview,
}

#[derive(Debug, Clone, Copy)]
pub struct UiHitboxes {
    pub map_inner: Rect,
    pub buttons: [Rect; 6],
    pub inspector_upgrade: Rect, // linha clicável no inspector
}

impl Default for UiHitboxes {
    fn default() -> Self {
        Self {
            map_inner: Rect::new(0, 0, 0, 0),
            buttons: [Rect::new(0, 0, 0, 0); 6],
            inspector_upgrade: Rect::new(0, 0, 0, 0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MapViewport {
    pub tile_w: u16, // colunas por célula (2)
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
    pub hit: UiHitboxes,
    pub viewport: MapViewport,
}

#[derive(Debug, Clone, Copy)]
pub enum TowerKind {
    Basic,
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
}

#[derive(Debug, Clone, Copy)]
pub enum ParticleKind {
    Trail,
    Spark,
    Smoke,
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

    tick_rate: Duration,
    last_tick: Instant,

    // RNG simples (sem deps)
    rng: u64,

    pub ui: UiState,
    pub game: GameState,
}

impl App {
    pub fn new() -> Self {
        let grid_w = 30;
        let grid_h = 16;

        // Path em “S”
        let mut path = Vec::new();
        for x in 1..(grid_w - 1) {
            path.push((x, 3));
        }
        for y in 3..(grid_h - 2) {
            path.push((grid_w - 2, y));
        }
        for x in (2..(grid_w - 1)).rev() {
            path.push((x, grid_h - 3));
        }

        let mut app = Self {
            should_quit: false,
            // tick mais curto -> animações mais suaves
            tick_rate: Duration::from_millis(50),
            last_tick: Instant::now(),
            rng: 0xC0FFEE_u64 ^ (Instant::now().elapsed().as_nanos() as u64),
            ui: UiState {
                mode: LayoutMode::Wide,
                hover_button: None,
                hover_cell: None,
                hover_action: None,
                hit: UiHitboxes::default(),
                viewport: MapViewport::default(),
            },
            game: GameState {
                running: false,
                speed: 1,
                money: 250,
                lives: 20,
                wave: 1,
                grid_w,
                grid_h,
                selected_cell: Some((6, 6)),
                path,
                towers: vec![Tower {
                    x: 6,
                    y: 6,
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

    pub fn on_tick_if_due(&mut self) {
        if self.last_tick.elapsed() >= self.tick_rate {
            self.last_tick = Instant::now();
            self.on_tick();
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

        if self.game.lives <= 0 {
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

            if e.move_cd > sp {
                e.move_cd -= sp;
                continue;
            }

            // move 1 tile
            e.move_cd = base;
            if e.path_i + 1 < self.game.path.len() {
                e.path_i += 1;
            } else {
                e.hp = 0;
                self.game.lives -= 1;
            }
        }
    }

    fn tick_towers(&mut self) {
        let sp = self.game.speed.max(1) as u16;
        let mut spawns: Vec<(u16, u16, u16, u16, i32)> = Vec::new();

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

            spawns.push((t.x, t.y, tx, ty, stats.attack));
            t.cooldown = stats.fire_cd;
        }

        for (from_x, from_y, to_x, to_y, dmg) in spawns {
            self.spawn_projectile(from_x, from_y, to_x, to_y, dmg);
        }
    }

    fn tick_projectiles(&mut self) {
        let sp = self.game.speed.max(1) as u16;
        let base_step_cd = self.projectile_base_step_cd();
        let mut trails: Vec<(i16, i16)> = Vec::new();
        let mut impacts: Vec<(u16, u16)> = Vec::new();

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
            p.step_cd = base_step_cd;

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
                }

                impacts.push((hit_x, hit_y));
                p.ttl = 0;
            }
        }

        self.game.projectiles.retain(|p| p.ttl > 0);

        for (x, y) in trails {
            self.spawn_trail(x, y);
        }
        for (x, y) in impacts {
            self.spawn_impact(x, y);
        }
    }

    fn tick_economy_and_waves(&mut self) {
        // dinheiro por segundo (não por tick)
        if self.game.money_cd > 0 {
            self.game.money_cd -= 1;
        } else {
            // ganho lento, pra combinar com ritmo mais "tático"
            self.game.money += 2;
            self.game.money_cd = 20; // 20 ticks * 50ms = 1s
        }

        let alive = self.game.enemies.iter().any(|e| e.hp > 0);
        if !alive && self.game.lives > 0 {
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
            });
        }

        // limpa FX antigos pra não virar bagunça visual ao trocar wave
        self.game.projectiles.clear();
        self.game.impacts.clear();
        self.game.particles.clear();
    }

    fn enemy_base_move_cd(&self) -> u16 {
        // 50ms por tick
        // 14 ticks = 700ms por tile (bem mais lento)
        14
    }

    fn projectile_base_step_cd(&self) -> u16 {
        // 2 ticks = 100ms por tile (dá tempo de ver a trilha)
        2
    }

    fn spawn_projectile(&mut self, from_x: u16, from_y: u16, to_x: u16, to_y: u16, dmg: i32) {
        self.game.projectiles.push(Projectile {
            x: from_x as i16,
            y: from_y as i16,
            tx: to_x as i16,
            ty: to_y as i16,
            ttl: 90,
            damage: dmg,
            step_cd: self.projectile_base_step_cd(),
        });

        // pequeno flash de "muzzle" na torre
        self.spawn_spark(from_x as i16, from_y as i16);
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

    fn spawn_impact(&mut self, x: u16, y: u16) {
        self.game.impacts.push(ImpactFx { x, y, ttl: 4 });

        let ix = x as i16;
        let iy = y as i16;

        // fagulhas
        for _ in 0..8 {
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
        self.game.particles.push(Particle {
            x: ix,
            y: iy,
            vx: 0,
            vy: 0,
            ttl: 10,
            kind: ParticleKind::Smoke,
        });
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
        Self::enemy_index_at(self.game.enemies.as_slice(), self.game.path.as_slice(), x, y).is_some()
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
        // Ajustado pra tick=50ms (antes era ~120ms):
        // lvl 1: atk 40, range 6, cd 18 (~900ms)
        // +20 atk por lvl, range +1 a cada 2 lvls,
        // cd -2 a cada 3 lvls (clamp)
        let lvl = t.level.max(1) as i32;
        let attack = 40 + (lvl - 1) * 20;

        let range = 6 + ((t.level.saturating_sub(1) / 2) as u16);
        let cd_base: i32 = 18 - ((t.level.saturating_sub(1) / 3) as i32) * 2;
        let fire_cd = cd_base.clamp(8, 24) as u16;

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

    fn try_build(&mut self) {
        let Some((x, y)) = self.game.selected_cell else { return; };
        if self.is_path(x, y) {
            return;
        }
        if self.tower_index_at(x, y).is_some() {
            return;
        }
        if self.game.money < 50 {
            return;
        }
        self.game.money -= 50;
        self.game.towers.push(Tower {
            x,
            y,
            kind: TowerKind::Basic,
            level: 1,
            cooldown: 0,
        });
    }

    fn try_upgrade(&mut self) {
        let Some((x, y)) = self.game.selected_cell else { return; };
        let Some(idx) = self.tower_index_at(x, y) else { return; };

        if self.game.money < 30 {
            return;
        }
        let t = &mut self.game.towers[idx];
        if t.level >= 9 {
            return;
        }

        self.game.money -= 30;
        t.level += 1;
    }

    fn try_sell(&mut self) {
        let Some((x, y)) = self.game.selected_cell else { return; };
        let Some(idx) = self.tower_index_at(x, y) else { return; };
        self.game.towers.remove(idx);
        self.game.money += 20;
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
