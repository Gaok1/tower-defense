use crate::{
    fx::{FxManager, Vec2i},
    net_msg::{FxEvent, GameSnapshot, NetCmd, NetMsg},
    save,
};
use p2p_connection::{P2pConfig, P2pEvent, P2pNode};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, to_vec};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(target_os = "windows")]
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    Multiplayer,
    MapSelect,
    LoadGame,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiplayerRole {
    Host,
    Peer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpMode {
    Ipv4,
    Ipv6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiplayerFocus {
    Role,
    IpMode,
    PublicIp,
    PeerIp,
    Connect,
    Name,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Idle,
    FetchingIp,
    Ready,
    Connecting,
    Connected,
    Failed,
}

#[derive(Debug, Clone)]
pub struct PlayerCursor {
    pub name: String,
    pub x: u16,
    pub y: u16,
    pub pending_build: Option<(u16, u16, TowerKind)>,
}

#[derive(Debug)]
pub struct MultiplayerState {
    pub active: bool,
    pub role: MultiplayerRole,
    pub ip_mode: IpMode,
    pub local_endpoint: Option<SocketAddr>,
    pub peer_ip: String,
    pub status: ConnectionStatus,
    pub focus: MultiplayerFocus,
    pub name_input: String,
    pub player_name: Option<String>,
    pub peer_name: Option<String>,
    pub last_error: Option<String>,
    pub last_info: Option<String>,
    pub cursors: Vec<PlayerCursor>,
    pub pending_fx: Vec<FxEvent>,
    pub network: Option<MultiplayerNetwork>,
    pub next_cmd_id: u32,
    pub peer_id: Option<String>,
    pub peer_disconnected_in_game: bool,
    pub reconnecting: bool,
    pub reconnect_until: Option<Instant>,
    pub reconnect_last_attempt: Option<Instant>,
    pub reconnect_peer_addr: Option<SocketAddr>,
    pub reconnect_was_running: bool,
}

impl MultiplayerState {
    fn new() -> Self {
        Self {
            active: false,
            role: MultiplayerRole::Host,
            ip_mode: IpMode::Ipv4,
            local_endpoint: None,
            peer_ip: String::new(),
            status: ConnectionStatus::Idle,
            focus: MultiplayerFocus::Role,
            name_input: String::new(),
            player_name: None,
            peer_name: None,
            last_error: None,
            last_info: None,
            cursors: Vec::new(),
            pending_fx: Vec::new(),
            network: None,
            next_cmd_id: 1,
            peer_id: None,
            peer_disconnected_in_game: false,
            reconnecting: false,
            reconnect_until: None,
            reconnect_last_attempt: None,
            reconnect_peer_addr: None,
            reconnect_was_running: false,
        }
    }

    fn current_bind_addr(&self) -> SocketAddr {
        let bind_ip = match self.ip_mode {
            IpMode::Ipv4 => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            IpMode::Ipv6 => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        };
        SocketAddr::new(bind_ip, 0)
    }

    fn ensure_network(&mut self) {
        let bind_addr = self.current_bind_addr();
        if let Some(net) = &self.network {
            if net.bind_addr == bind_addr {
                return;
            }
        }
        self.shutdown_network();

        let (node_tx, node_rx) = mpsc::channel::<P2pNode>();
        let (evt_std_tx, evt_std_rx) = mpsc::channel::<P2pEvent>();
        let _thread = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let cfg = P2pConfig { bind_addr, ..Default::default() };
                let (node, mut evt_rx) = match P2pNode::start(cfg).await {
                    Ok(pair) => pair,
                    Err(_) => return,
                };
                let _ = node_tx.send(node);
                while let Some(evt) = evt_rx.recv().await {
                    if evt_std_tx.send(evt).is_err() {
                        break;
                    }
                }
            });
        });
        match node_rx.recv() {
            Ok(node) => {
                self.network = Some(MultiplayerNetwork {
                    node,
                    evt_rx: evt_std_rx,
                    _thread,
                    bind_addr,
                    peer_addr: None,
                });
            }
            Err(_) => {
                self.status = ConnectionStatus::Failed;
                self.last_error = Some("falha ao iniciar rede".to_string());
            }
        }
    }

    fn shutdown_network(&mut self) {
        if let Some(net) = self.network.take() {
            net.node.shutdown();
        }
    }

    fn queue_game_msg(&mut self, msg: &NetMsg) -> Result<(), String> {
        let payload = to_vec(msg).map_err(|e| e.to_string())?;
        let net = self.network.as_ref().ok_or("rede indisponivel")?;
        if net.node.broadcast_data(payload) {
            Ok(())
        } else {
            Err("canal fechado".to_string())
        }
    }

    fn refresh_network(&mut self) {
        self.shutdown_network();
        self.ensure_network();
    }
}

pub(crate) struct MultiplayerNetwork {
    node: P2pNode,
    evt_rx: mpsc::Receiver<P2pEvent>,
    _thread: thread::JoinHandle<()>,
    bind_addr: SocketAddr,
    peer_addr: Option<SocketAddr>,
}

impl std::fmt::Debug for MultiplayerNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiplayerNetwork")
            .field("bind_addr", &self.bind_addr)
            .field("peer_addr", &self.peer_addr)
            .finish()
    }
}

pub const TOWER_KIND_COUNT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonId {
    StartPause,
    StartWave,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiplayerAction {
    CreateLobby,
    JoinLobby,
    ToggleIpMode,
    CopyStunIp,
    RefreshStun,
    Connect,
    Continue,
    FocusPeerIp,
    FocusName,
    KickPlayer(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct UiHitboxes {
    pub map_inner: Rect,
    pub buttons: [Rect; 7],
    pub inspector_upgrade: Rect, // linha clicável no inspector
    pub build_options: [Rect; TOWER_KIND_COUNT],
    pub map_select_left: Rect,
    pub map_select_right: Rect,
    pub map_select_start: Rect,
}

pub const MAIN_MENU_OPTION_COUNT: usize = 3;

#[derive(Debug, Clone, Copy)]
pub struct MainMenuHitboxes {
    pub options: [Rect; MAIN_MENU_OPTION_COUNT],
}

impl Default for MainMenuHitboxes {
    fn default() -> Self {
        Self {
            options: [Rect::new(0, 0, 0, 0); MAIN_MENU_OPTION_COUNT],
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadMenuHitboxes {
    pub slot_items: Vec<Rect>,
    pub wave_items: Vec<Rect>,
}

impl Default for LoadMenuHitboxes {
    fn default() -> Self {
        Self {
            slot_items: Vec::new(),
            wave_items: Vec::new(),
        }
    }
}

impl Default for UiHitboxes {
    fn default() -> Self {
        Self {
            map_inner: Rect::new(0, 0, 0, 0),
            buttons: [Rect::new(0, 0, 0, 0); 7],
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
pub struct MultiplayerHitboxes {
    pub create_btn: Rect,
    pub join_btn: Rect,
    pub ip_mode_btn: Rect,
    pub copy_ip_btn: Rect,
    pub refresh_ip_btn: Rect,
    pub connect_btn: Rect,
    pub continue_btn: Rect,
    pub peer_ip_field: Rect,
    pub name_field: Rect,
    pub kick_buttons: Vec<Rect>,
    pub kick_targets: Vec<usize>,
}

impl Default for MultiplayerHitboxes {
    fn default() -> Self {
        Self {
            create_btn: Rect::new(0, 0, 0, 0),
            join_btn: Rect::new(0, 0, 0, 0),
            ip_mode_btn: Rect::new(0, 0, 0, 0),
            copy_ip_btn: Rect::new(0, 0, 0, 0),
            refresh_ip_btn: Rect::new(0, 0, 0, 0),
            connect_btn: Rect::new(0, 0, 0, 0),
            continue_btn: Rect::new(0, 0, 0, 0),
            peer_ip_field: Rect::new(0, 0, 0, 0),
            name_field: Rect::new(0, 0, 0, 0),
            kick_buttons: Vec::new(),
            kick_targets: Vec::new(),
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
    pub hover_multiplayer: Option<MultiplayerAction>,
    pub hover_main_menu: Option<usize>,
    pub hover_load_slot: Option<usize>,
    pub hover_load_wave: Option<usize>,
    pub top_notice: Option<TopNotice>,
    pub hit: UiHitboxes,
    pub multiplayer_hit: MultiplayerHitboxes,
    pub main_menu_hit: MainMenuHitboxes,
    pub load_menu_hit: LoadMenuHitboxes,
    pub viewport: MapViewport,
    pub zoom: u16,
    pub last_zoom: u16, // <-- NOVO (pra ancorar zoom sem "pular")
    pub manual_pan: bool,
    pub drag_origin: Option<(u16, u16)>,
    pub drag_view: Option<(u16, u16)>,
    pub anim_tick: u32,
}

#[derive(Debug, Clone)]
pub struct TopNotice {
    pub text: String,
    pub ttl_ticks: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TowerKind {
    Basic,
    Sniper,
    Rapid,
    Cannon,
    Tesla,
    Frost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyKind {
    Runner,
    Tank,
    Swarm,
    Shielded,
    Healer,
    Sneak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetMode {
    Primeiro,
    Ultimo,
    MaisForte,
    MaisFraco,
    MaisRapido,
    MaisLento,
    MaisPerigoso,
    MaisCurador,
}

impl TargetMode {
    fn next(self) -> Self {
        match self {
            TargetMode::Primeiro => TargetMode::Ultimo,
            TargetMode::Ultimo => TargetMode::MaisForte,
            TargetMode::MaisForte => TargetMode::MaisFraco,
            TargetMode::MaisFraco => TargetMode::MaisRapido,
            TargetMode::MaisRapido => TargetMode::MaisLento,
            TargetMode::MaisLento => TargetMode::MaisPerigoso,
            TargetMode::MaisPerigoso => TargetMode::MaisCurador,
            TargetMode::MaisCurador => TargetMode::Primeiro,
        }
    }
}

impl From<save::TargetModeSave> for TargetMode {
    fn from(value: save::TargetModeSave) -> Self {
        match value {
            save::TargetModeSave::Primeiro => TargetMode::Primeiro,
            save::TargetModeSave::Ultimo => TargetMode::Ultimo,
            save::TargetModeSave::MaisForte => TargetMode::MaisForte,
            save::TargetModeSave::MaisFraco => TargetMode::MaisFraco,
            save::TargetModeSave::MaisRapido => TargetMode::MaisRapido,
            save::TargetModeSave::MaisLento => TargetMode::MaisLento,
            save::TargetModeSave::MaisPerigoso => TargetMode::MaisPerigoso,
            save::TargetModeSave::MaisCurador => TargetMode::MaisCurador,
        }
    }
}

impl From<TargetMode> for save::TargetModeSave {
    fn from(value: TargetMode) -> Self {
        match value {
            TargetMode::Primeiro => save::TargetModeSave::Primeiro,
            TargetMode::Ultimo => save::TargetModeSave::Ultimo,
            TargetMode::MaisForte => save::TargetModeSave::MaisForte,
            TargetMode::MaisFraco => save::TargetModeSave::MaisFraco,
            TargetMode::MaisRapido => save::TargetModeSave::MaisRapido,
            TargetMode::MaisLento => save::TargetModeSave::MaisLento,
            TargetMode::MaisPerigoso => save::TargetModeSave::MaisPerigoso,
            TargetMode::MaisCurador => save::TargetModeSave::MaisCurador,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tower {
    pub x: u16,
    pub y: u16,
    pub kind: TowerKind,
    pub level: u8,
    pub cooldown: u16, // ticks até poder atirar
    pub target_mode: TargetMode,
    #[serde(default)]
    pub fire_age: u8, // contagem regressiva pós-disparo (10→0), 0 = idle
    #[serde(default)]
    pub fire_dir: (i16, i16), // direção normalizada do último tiro
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub path_i: usize,
    pub hp: i32,
    pub move_cd: u16, // ticks até o próximo passo
    pub slow_ticks: u16,
    pub slow_percent: u8,
    pub reward: i32,
    pub rewarded: bool,
    pub escaped: bool,
}

#[derive(Debug, Clone, Copy)]
struct EnemyTuning {
    base_hp: i32,
    move_cd: u16,
    slow_resist: u8,
    splash_resist: u8,
    rapid_resist: u8,
    heal_radius: u16,
    heal_amount: i32,
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
    pub fx_id: Option<usize>,
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
    pub fx: FxManager,

    // economia/waves
    pub pending_wave_start: bool,
    pub prep_ticks: u32,
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
    pub multiplayer: MultiplayerState,

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
    fn show_top_notice(&mut self, text: impl Into<String>) {
        self.ui.top_notice = Some(TopNotice {
            text: text.into(),
            ttl_ticks: 60,
        });
    }

    fn tick_top_notice(&mut self) {
        if let Some(notice) = self.ui.top_notice.as_mut() {
            notice.ttl_ticks = notice.ttl_ticks.saturating_sub(1);
            if notice.ttl_ticks == 0 {
                self.ui.top_notice = None;
            }
        }
    }

    fn tick_reconnect(&mut self) {
        if !self.multiplayer.reconnecting {
            return;
        }

        let timed_out = self
            .multiplayer
            .reconnect_until
            .map(|t| Instant::now() >= t)
            .unwrap_or(true);

        if timed_out {
            self.multiplayer.reconnecting = false;
            self.multiplayer.reconnect_until = None;
            self.multiplayer.reconnect_last_attempt = None;
            self.multiplayer.reconnect_peer_addr = None;
            self.multiplayer.peer_disconnected_in_game = true;
            self.multiplayer.active = false;
            self.multiplayer.status = ConnectionStatus::Ready;
            self.game.running = true;
            self.show_top_notice("Conexão perdida.".to_string());
            return;
        }

        let should_retry = self
            .multiplayer
            .reconnect_last_attempt
            .map(|t| t.elapsed() >= Duration::from_secs(3))
            .unwrap_or(true);

        if should_retry {
            if let (Some(addr), Some(net)) = (
                self.multiplayer.reconnect_peer_addr,
                self.multiplayer.network.as_ref(),
            ) {
                net.node.connect_peer(addr);
                self.multiplayer.reconnect_last_attempt = Some(Instant::now());
            }
        }
    }

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
            multiplayer: MultiplayerState::new(),
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
                hover_multiplayer: None,
                hover_main_menu: None,
                hover_load_slot: None,
                hover_load_wave: None,
                top_notice: None,
                hit: UiHitboxes::default(),
                multiplayer_hit: MultiplayerHitboxes::default(),
                main_menu_hit: MainMenuHitboxes::default(),
                load_menu_hit: LoadMenuHitboxes::default(),
                viewport: MapViewport::default(),
                zoom: 1,
                last_zoom: 1, // <-- NOVO
                manual_pan: false,
                drag_origin: None,
                drag_view: None,
                anim_tick: 0,
            },

            game: GameState {
                running: false,
                speed: 1,
                money: 175,
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
                    target_mode: TargetMode::Primeiro,
                    fire_age: 0,
                    fire_dir: (0, 0),
                }],
                enemies: vec![],
                projectiles: vec![],
                fx: FxManager::new(),
                pending_wave_start: false,
                prep_ticks: 0,
            },
            maps,
            map_index,
        };

        app.spawn_wave(false);
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

    pub fn enter_multiplayer_menu(&mut self) {
        self.multiplayer = MultiplayerState::new();
        self.multiplayer.status = ConnectionStatus::FetchingIp;
        self.multiplayer.ensure_network();
        self.screen = Screen::Multiplayer;
    }

    pub fn main_menu_prev(&mut self) {
        const COUNT: usize = 3; // New game / Load game / Multiplayer
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
        const COUNT: usize = 3; // New game / Load game / Multiplayer
        if COUNT == 0 {
            return;
        }
        self.main_menu_index = (self.main_menu_index + 1) % COUNT;
    }

    pub fn main_menu_activate(&mut self) {
        match self.main_menu_index {
            0 => {
                self.multiplayer.active = false;
                self.multiplayer.shutdown_network();
                self.screen = Screen::MapSelect;
            }
            1 => {
                self.multiplayer.active = false;
                self.multiplayer.shutdown_network();
                self.enter_load_game();
            }
            2 => self.enter_multiplayer_menu(),
            _ => {}
        }
    }

    pub fn enter_main_menu(&mut self) {
        self.screen = Screen::MainMenu;
        self.multiplayer.active = false;
        self.multiplayer.peer_disconnected_in_game = false;
        self.multiplayer.reconnecting = false;
        self.multiplayer.reconnect_until = None;
        self.multiplayer.reconnect_last_attempt = None;
        self.multiplayer.reconnect_peer_addr = None;
        self.multiplayer.reconnect_was_running = false;
        self.multiplayer.shutdown_network();
        self.multiplayer.cursors.clear();
        self.ui.hover_main_menu = None;
        self.ui.hover_load_slot = None;
        self.ui.hover_load_wave = None;
    }

    pub fn enter_load_game(&mut self) {
        self.multiplayer.active = false;
        self.multiplayer.shutdown_network();
        self.refresh_load_menu();
        self.screen = Screen::LoadGame;
        self.ui.hover_load_slot = None;
        self.ui.hover_load_wave = None;
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

    pub fn multiplayer_focus_prev(&mut self) {
        self.multiplayer.focus = match self.multiplayer.focus {
            MultiplayerFocus::Role => MultiplayerFocus::Continue,
            MultiplayerFocus::IpMode => MultiplayerFocus::Role,
            MultiplayerFocus::PublicIp => MultiplayerFocus::IpMode,
            MultiplayerFocus::PeerIp => MultiplayerFocus::PublicIp,
            MultiplayerFocus::Connect => MultiplayerFocus::PeerIp,
            MultiplayerFocus::Name => MultiplayerFocus::Connect,
            MultiplayerFocus::Continue => MultiplayerFocus::Name,
        };
    }

    pub fn multiplayer_focus_next(&mut self) {
        self.multiplayer.focus = match self.multiplayer.focus {
            MultiplayerFocus::Role => MultiplayerFocus::IpMode,
            MultiplayerFocus::IpMode => MultiplayerFocus::PublicIp,
            MultiplayerFocus::PublicIp => MultiplayerFocus::PeerIp,
            MultiplayerFocus::PeerIp => MultiplayerFocus::Connect,
            MultiplayerFocus::Connect => MultiplayerFocus::Name,
            MultiplayerFocus::Name => MultiplayerFocus::Continue,
            MultiplayerFocus::Continue => MultiplayerFocus::Role,
        };
    }

    pub fn multiplayer_toggle_role(&mut self) {
        let role = match self.multiplayer.role {
            MultiplayerRole::Host => MultiplayerRole::Peer,
            MultiplayerRole::Peer => MultiplayerRole::Host,
        };
        self.multiplayer_set_role(role);
    }

    pub fn multiplayer_set_role(&mut self, role: MultiplayerRole) {
        self.multiplayer.role = role;
        self.multiplayer.last_error = None;
        self.multiplayer.last_info = None;
        self.multiplayer.focus = match role {
            MultiplayerRole::Host => MultiplayerFocus::PublicIp,
            MultiplayerRole::Peer => MultiplayerFocus::PeerIp,
        };
    }

    pub fn multiplayer_toggle_ip_mode(&mut self) {
        self.multiplayer.ip_mode = match self.multiplayer.ip_mode {
            IpMode::Ipv4 => IpMode::Ipv6,
            IpMode::Ipv6 => IpMode::Ipv4,
        };
        self.multiplayer.status = ConnectionStatus::FetchingIp;
        self.multiplayer.local_endpoint = None;
        self.multiplayer.last_error = None;
        self.multiplayer.last_info = None;
        self.multiplayer.refresh_network();
    }

    pub fn multiplayer_refresh_ip(&mut self) {
        self.multiplayer.status = ConnectionStatus::FetchingIp;
        self.multiplayer.local_endpoint = None;
        self.multiplayer.last_error = None;
        self.multiplayer.last_info = None;
        self.multiplayer.refresh_network();
    }

    pub fn multiplayer_copy_public_ip(&mut self) {
        self.multiplayer.last_error = None;
        self.multiplayer.last_info = None;

        let Some(endpoint) = self.multiplayer.local_endpoint.as_ref() else {
            self.multiplayer.last_error = Some("Codigo ainda nao esta disponivel".to_string());
            return;
        };

        let endpoint_text = endpoint.to_string();
        match copy_to_clipboard(&endpoint_text) {
            Ok(()) => {
                self.multiplayer.last_info =
                    Some("Codigo copiado para a area de transferencia".to_string());
            }
            Err(e) => {
                self.multiplayer.last_error = Some(format!("falha ao copiar codigo: {e}"));
            }
        }
    }

    pub fn multiplayer_connect(&mut self) {
        self.multiplayer.last_error = None;
        self.multiplayer.last_info = None;
        self.multiplayer.ensure_network();

        if self.multiplayer.peer_ip.trim().is_empty() {
            self.multiplayer.status = ConnectionStatus::Failed;
            self.multiplayer.last_error = Some("Cole o codigo do outro jogador".to_string());
            return;
        }

        let Some(addr) = self.parse_peer_addr() else {
            self.multiplayer.status = ConnectionStatus::Failed;
            self.multiplayer.last_error = Some("Codigo invalido. Ex: 127.0.0.1:5000".to_string());
            return;
        };

        self.multiplayer.active = false;
        self.multiplayer.peer_name = None;
        self.multiplayer.cursors.clear();

        let connected = if let Some(net) = self.multiplayer.network.as_ref() {
            net.node.connect_peer(addr)
        } else {
            false
        };
        if !connected {
            self.multiplayer.status = ConnectionStatus::Failed;
            self.multiplayer.last_error = Some("rede indisponivel".to_string());
            return;
        }

        self.multiplayer.status = ConnectionStatus::Connecting;
        self.multiplayer.last_info = Some(match self.multiplayer.role {
            MultiplayerRole::Host => {
                if self.dev_mode {
                    format!("conectando em {addr}")
                } else {
                    "conectando...".to_string()
                }
            }
            MultiplayerRole::Peer => {
                if self.dev_mode {
                    format!("conectando em {addr}")
                } else {
                    "conectando...".to_string()
                }
            }
        });
    }

    pub fn multiplayer_kick_player(&mut self, index: usize) {
        if self.multiplayer.role != MultiplayerRole::Host {
            self.multiplayer.last_error = Some("apenas o host pode remover jogadores".to_string());
            return;
        }
        if self.multiplayer.status != ConnectionStatus::Connected {
            self.multiplayer.last_error = Some("nenhum jogador conectado ainda".to_string());
            return;
        }
        if index == 0 {
            self.multiplayer.last_error = Some("voce nao pode remover voce mesmo".to_string());
            return;
        }
        let msg = NetMsg::Kick {
            reason: Some("removido pelo host".to_string()),
        };
        if let Err(err) = self.multiplayer.queue_game_msg(&msg) {
            self.multiplayer.status = ConnectionStatus::Failed;
            self.multiplayer.last_error = Some(err);
            return;
        }
        self.multiplayer.last_info = Some("jogador removido".to_string());
    }

    pub fn multiplayer_continue(&mut self) {
        if self.multiplayer.status != ConnectionStatus::Connected {
            return;
        }
        if self.multiplayer.name_input.trim().is_empty() {
            self.multiplayer.last_error = Some("digite seu nome para continuar".to_string());
            self.multiplayer.last_info = None;
            return;
        }
        self.multiplayer.player_name = Some(self.multiplayer.name_input.trim().to_string());
        self.multiplayer.last_error = None;
        self.multiplayer.last_info = None;

        let msg = NetMsg::Hello {
            name: self.multiplayer.name_input.trim().to_string(),
        };
        if let Err(err) = self.multiplayer.queue_game_msg(&msg) {
            self.multiplayer.status = ConnectionStatus::Failed;
            self.multiplayer.last_error = Some(err);
            return;
        }
        if self.multiplayer.role == MultiplayerRole::Host {
            let enter = NetMsg::EnterMapSelect {
                map_index: self.map_index,
            };
            if let Err(err) = self.multiplayer.queue_game_msg(&enter) {
                self.multiplayer.status = ConnectionStatus::Failed;
                self.multiplayer.last_error = Some(err);
                return;
            }
            self.screen = Screen::MapSelect;
        } else {
            self.multiplayer.last_info = Some("nome enviado. aguardando o host...".to_string());
        }
    }

    pub fn multiplayer_input_char(&mut self, ch: char) {
        match self.multiplayer.focus {
            MultiplayerFocus::PeerIp => {
                if ch.is_ascii_digit() || ch == '.' || ch == ':' || ch == '[' || ch == ']' {
                    self.multiplayer.peer_ip.push(ch);
                }
            }
            MultiplayerFocus::Name => {
                if self.multiplayer.status == ConnectionStatus::Connected && !ch.is_control() {
                    self.multiplayer.name_input.push(ch);
                }
            }
            _ => {}
        }
    }

    pub fn multiplayer_backspace(&mut self) {
        match self.multiplayer.focus {
            MultiplayerFocus::PeerIp => {
                self.multiplayer.peer_ip.pop();
            }
            MultiplayerFocus::Name => {
                self.multiplayer.name_input.pop();
            }
            _ => {}
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

    fn poll_network_events(&mut self) {
        loop {
            let event = {
                let net = match self.multiplayer.network.as_mut() {
                    Some(net) => net,
                    None => return,
                };

                match net.evt_rx.try_recv() {
                    Ok(event) => event,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.multiplayer.status = ConnectionStatus::Failed;
                        self.multiplayer.last_error = Some(if self.dev_mode {
                            "canal de rede encerrado".to_string()
                        } else {
                            "conexao encerrada.".to_string()
                        });
                        self.multiplayer.network = None;
                        return;
                    }
                }
            };

            self.handle_net_event(event);
        }
    }

    fn handle_net_event(&mut self, event: P2pEvent) {
        match event {
            P2pEvent::Bound(addr) => {
                if self.dev_mode {
                    self.multiplayer.last_info = Some(format!("ouvindo {addr}"));
                }
            }
            P2pEvent::PublicEndpoint(addr) => {
                self.multiplayer.local_endpoint = Some(addr);
                if self.multiplayer.status == ConnectionStatus::FetchingIp {
                    self.multiplayer.status = ConnectionStatus::Ready;
                    self.multiplayer.last_info = Some(if self.dev_mode {
                        format!("codigo pronto: {addr}")
                    } else {
                        "Codigo pronto. Copie e envie para o amigo.".to_string()
                    });
                }
            }
            P2pEvent::ObservedEndpoint(addr) => {
                if self.dev_mode {
                    self.multiplayer.last_info = Some(format!("endpoint observado {addr}"));
                }
            }
            P2pEvent::PeerConnecting(peer) => {
                self.multiplayer.last_info = Some(if self.dev_mode {
                    format!("conectando em {peer}")
                } else {
                    "conectando...".to_string()
                });
            }
            P2pEvent::PeerConnected(peer) => {
                // QUIC connection established — wait for PeerVerified before marking Connected
                self.multiplayer.last_info = Some(if self.dev_mode {
                    format!("peer conectado (aguardando auth): {peer}")
                } else {
                    "autenticando...".to_string()
                });
            }
            P2pEvent::PeerVerified { peer, peer_id } => {
                if let Some(net) = self.multiplayer.network.as_mut() {
                    net.peer_addr = Some(peer);
                }
                self.multiplayer.peer_id = Some(peer_id);
                self.multiplayer.status = ConnectionStatus::Connected;
                self.multiplayer.active = true;
                self.multiplayer.last_error = None;
                self.multiplayer.peer_disconnected_in_game = false;
                self.multiplayer.last_info = Some(if self.dev_mode {
                    format!("peer verificado: {peer}")
                } else {
                    "conectado!".to_string()
                });
                self.ensure_cursor_slots();

                if self.multiplayer.reconnecting {
                    self.multiplayer.reconnecting = false;
                    self.multiplayer.reconnect_until = None;
                    self.multiplayer.reconnect_last_attempt = None;
                    self.multiplayer.reconnect_peer_addr = None;
                    if self.screen == Screen::Game {
                        self.game.running = self.multiplayer.reconnect_was_running;
                        self.send_multiplayer_state();
                        let name = self
                            .multiplayer
                            .peer_name
                            .clone()
                            .unwrap_or_else(|| "Peer".to_string());
                        self.show_top_notice(format!("{name} reconectou!"));
                    }
                    self.multiplayer.reconnect_was_running = false;
                } else if self.multiplayer.role == MultiplayerRole::Host {
                    let hello = NetMsg::Hello {
                        name: self.multiplayer.name_input.trim().to_string(),
                    };
                    if let Err(err) = self.multiplayer.queue_game_msg(&hello) {
                        self.multiplayer.status = ConnectionStatus::Failed;
                        self.multiplayer.last_error = Some(err);
                        return;
                    }
                    let set_map = NetMsg::SetMap {
                        map_index: self.map_index,
                    };
                    if let Err(err) = self.multiplayer.queue_game_msg(&set_map) {
                        self.multiplayer.status = ConnectionStatus::Failed;
                        self.multiplayer.last_error = Some(err);
                        return;
                    }
                }
            }
            P2pEvent::PeerAuthFailed { peer, reason } => {
                self.multiplayer.status = ConnectionStatus::Failed;
                self.multiplayer.last_error = Some(if self.dev_mode {
                    format!("autenticacao falhou ({peer}): {reason}")
                } else {
                    "falha de autenticacao.".to_string()
                });
            }
            P2pEvent::PeerDisconnected(peer) => {
                let disconnected_name = self.multiplayer.peer_name.clone().unwrap_or_else(|| {
                    if self.multiplayer.role == MultiplayerRole::Peer {
                        "Host".to_string()
                    } else {
                        "Player".to_string()
                    }
                });
                self.multiplayer.peer_id = None;
                let saved_addr = self.multiplayer.network.as_ref().and_then(|n| n.peer_addr);
                if let Some(net) = self.multiplayer.network.as_mut() {
                    net.peer_addr = None;
                }

                if self.multiplayer.role == MultiplayerRole::Host && self.screen == Screen::Game {
                    self.multiplayer.active = false;
                    self.multiplayer.peer_name = None;
                    self.multiplayer.cursors.clear();
                    self.multiplayer.last_error = None;

                    if let Some(addr) = saved_addr {
                        self.multiplayer.reconnecting = true;
                        self.multiplayer.reconnect_until =
                            Some(Instant::now() + Duration::from_secs(30));
                        self.multiplayer.reconnect_peer_addr = Some(addr);
                        self.multiplayer.reconnect_last_attempt = Some(Instant::now());
                        self.multiplayer.reconnect_was_running = self.game.running;
                        self.multiplayer.status = ConnectionStatus::Connecting;
                        self.game.running = false;
                        if let Some(net) = self.multiplayer.network.as_ref() {
                            net.node.connect_peer(addr);
                        }
                    } else {
                        self.multiplayer.peer_disconnected_in_game = true;
                        self.multiplayer.status = ConnectionStatus::Ready;
                        self.multiplayer.last_info = Some(if self.dev_mode {
                            format!("jogador saiu: {peer}")
                        } else {
                            "jogador saiu da sala.".to_string()
                        });
                        self.show_top_notice(format!("{disconnected_name} desconectou."));
                    }
                } else {
                    self.multiplayer.active = false;
                    self.multiplayer.peer_name = None;
                    self.multiplayer.cursors.clear();
                    if self.multiplayer.role == MultiplayerRole::Host {
                        self.multiplayer.status = ConnectionStatus::Ready;
                        self.multiplayer.last_info = Some(if self.dev_mode {
                            format!("jogador saiu: {peer}")
                        } else {
                            "jogador saiu da sala.".to_string()
                        });
                        self.multiplayer.last_error = None;
                    } else {
                        self.multiplayer.status = ConnectionStatus::Failed;
                        self.multiplayer.last_error = Some(if self.dev_mode {
                            format!("peer desconectado: {peer}")
                        } else {
                            "conexao perdida.".to_string()
                        });
                    }
                    self.show_top_notice(format!("{disconnected_name} foi desconectado."));
                    if self.multiplayer.role == MultiplayerRole::Peer {
                        if self.screen == Screen::Game {
                            self.enter_main_menu();
                        } else {
                            self.screen = Screen::Multiplayer;
                            self.multiplayer.focus = MultiplayerFocus::Continue;
                        }
                    }
                }
            }
            P2pEvent::PeerTimeout(peer) => {
                let disconnected_name = self.multiplayer.peer_name.clone().unwrap_or_else(|| {
                    if self.multiplayer.role == MultiplayerRole::Peer {
                        "Host".to_string()
                    } else {
                        "Player".to_string()
                    }
                });
                self.multiplayer.peer_id = None;
                let saved_addr = self.multiplayer.network.as_ref().and_then(|n| n.peer_addr);
                if let Some(net) = self.multiplayer.network.as_mut() {
                    net.peer_addr = None;
                }

                if self.multiplayer.role == MultiplayerRole::Host && self.screen == Screen::Game {
                    self.multiplayer.active = false;
                    self.multiplayer.peer_name = None;
                    self.multiplayer.cursors.clear();
                    self.multiplayer.last_error = None;

                    if let Some(addr) = saved_addr {
                        self.multiplayer.reconnecting = true;
                        self.multiplayer.reconnect_until =
                            Some(Instant::now() + Duration::from_secs(30));
                        self.multiplayer.reconnect_peer_addr = Some(addr);
                        self.multiplayer.reconnect_last_attempt = Some(Instant::now());
                        self.multiplayer.reconnect_was_running = self.game.running;
                        self.multiplayer.status = ConnectionStatus::Connecting;
                        self.game.running = false;
                        if let Some(net) = self.multiplayer.network.as_ref() {
                            net.node.connect_peer(addr);
                        }
                    } else {
                        self.multiplayer.peer_disconnected_in_game = true;
                        self.multiplayer.status = ConnectionStatus::Ready;
                        self.multiplayer.last_info = Some(if self.dev_mode {
                            format!("tempo esgotado {peer}")
                        } else {
                            "tempo esgotado.".to_string()
                        });
                        self.show_top_notice(format!("{disconnected_name} desconectou (timeout)."));
                    }
                } else {
                    self.multiplayer.active = false;
                    self.multiplayer.peer_name = None;
                    self.multiplayer.cursors.clear();
                    if self.multiplayer.role == MultiplayerRole::Host {
                        self.multiplayer.status = ConnectionStatus::Ready;
                        self.multiplayer.last_info = Some(if self.dev_mode {
                            format!("tempo esgotado {peer}")
                        } else {
                            "tempo esgotado.".to_string()
                        });
                        self.multiplayer.last_error = None;
                    } else {
                        self.multiplayer.status = ConnectionStatus::Failed;
                        self.multiplayer.last_error = Some(if self.dev_mode {
                            format!("tempo esgotado {peer}")
                        } else {
                            "tempo esgotado.".to_string()
                        });
                    }
                    self.show_top_notice(format!("{disconnected_name} foi desconectado."));
                    if self.multiplayer.role == MultiplayerRole::Peer {
                        if self.screen == Screen::Game {
                            self.enter_main_menu();
                        } else {
                            self.screen = Screen::Multiplayer;
                            self.multiplayer.focus = MultiplayerFocus::Continue;
                        }
                    }
                }
            }
            P2pEvent::Log(msg) => {
                if msg.to_lowercase().contains("erro") {
                    self.multiplayer.last_error = Some(if self.dev_mode {
                        msg
                    } else {
                        "falha na conexao.".to_string()
                    });
                    if self.multiplayer.status == ConnectionStatus::FetchingIp {
                        self.multiplayer.status = ConnectionStatus::Failed;
                    }
                } else if self.dev_mode {
                    self.multiplayer.last_info = Some(msg);
                }
            }
            P2pEvent::DataReceived { payload, .. } => match from_slice(&payload) {
                Ok(net_msg) => self.handle_net_msg(net_msg),
                Err(err) => {
                    self.multiplayer.last_error = Some(if self.dev_mode {
                        format!("falha ao decodificar mensagem: {err}")
                    } else {
                        "falha ao receber dados do multiplayer.".to_string()
                    });
                }
            },
            _ => {}
        }
    }

    fn handle_net_msg(&mut self, msg: NetMsg) {
        match msg {
            NetMsg::Hello { name } => {
                if name.trim().is_empty() {
                    return;
                }
                self.multiplayer.peer_name = Some(name);
                // B2: Peer responds with its own Hello if not yet sent
                if self.multiplayer.role == MultiplayerRole::Peer
                    && !self.multiplayer.name_input.trim().is_empty()
                    && self.multiplayer.player_name.is_none()
                {
                    let my_name = self.multiplayer.name_input.trim().to_string();
                    self.multiplayer.player_name = Some(my_name.clone());
                    let reply = NetMsg::Hello { name: my_name };
                    let _ = self.multiplayer.queue_game_msg(&reply);
                }
                self.ensure_cursor_slots();
            }
            NetMsg::Kick { reason } => {
                if self.multiplayer.role != MultiplayerRole::Peer {
                    return;
                }
                self.multiplayer.active = false;
                self.multiplayer.status = ConnectionStatus::Failed;
                self.multiplayer.last_error =
                    Some(reason.unwrap_or_else(|| "expulso do lobby".to_string()));
                self.multiplayer.shutdown_network();
                self.screen = Screen::Multiplayer;
                self.multiplayer.focus = MultiplayerFocus::Continue;
            }
            NetMsg::EnterMapSelect { map_index } => {
                if self.multiplayer.role != MultiplayerRole::Peer {
                    return;
                }
                if !self.multiplayer.active
                    || self.multiplayer.status != ConnectionStatus::Connected
                {
                    return;
                }
                if self.maps.is_empty() {
                    return;
                }
                self.map_index = map_index.min(self.maps.len().saturating_sub(1));
                self.screen = Screen::MapSelect;
            }
            NetMsg::SetMap { map_index } => {
                if self.multiplayer.role != MultiplayerRole::Peer {
                    return;
                }
                if self.maps.is_empty() {
                    return;
                }
                self.map_index = map_index.min(self.maps.len().saturating_sub(1));
            }
            NetMsg::StartGame { map_index } => {
                if self.multiplayer.role != MultiplayerRole::Peer {
                    return;
                }
                self.start_multiplayer_game_from_host(map_index);
            }
            NetMsg::Cursor { name, x, y, pending_build } => {
                if !self.multiplayer.active {
                    return;
                }
                self.ensure_cursor_slots();
                if self.multiplayer.role == MultiplayerRole::Host {
                    if let Some(peer) = self.multiplayer.cursors.get_mut(1) {
                        peer.name = name;
                        peer.x = x;
                        peer.y = y;
                        peer.pending_build = pending_build;
                    }
                } else if let Some(peer) = self.multiplayer.cursors.get_mut(1) {
                    peer.name = name;
                    peer.x = x;
                    peer.y = y;
                    peer.pending_build = pending_build;
                }
            }
            NetMsg::Cmd { id, cmd } => {
                if self.multiplayer.role != MultiplayerRole::Host {
                    return;
                }
                let res = self.apply_net_cmd(cmd);
                let (ok, error) = match res {
                    Ok(()) => (true, None),
                    Err(e) => (false, Some(e)),
                };
                let result_msg = NetMsg::CmdResult { id, ok, error };
                if let Err(err) = self.multiplayer.queue_game_msg(&result_msg) {
                    self.multiplayer.status = ConnectionStatus::Failed;
                    self.multiplayer.last_error = Some(err);
                }
            }
            NetMsg::CmdResult { id: _, ok, error } => {
                if self.multiplayer.role != MultiplayerRole::Peer {
                    return;
                }
                if !ok {
                    self.multiplayer.last_error =
                        Some(error.unwrap_or_else(|| "falha".to_string()));
                }
                // B3: do NOT clear last_error on success — may be from another source
            }
            NetMsg::State { state } => {
                if self.multiplayer.role != MultiplayerRole::Peer {
                    return;
                }
                self.apply_snapshot(state);
            }
            NetMsg::Fx { events } => {
                if self.multiplayer.role != MultiplayerRole::Peer {
                    return;
                }
                self.apply_fx_events(events);
            }
        }
    }

    fn apply_net_cmd(&mut self, cmd: NetCmd) -> Result<(), String> {
        if self.screen != Screen::Game {
            return Err("host ainda nao iniciou o jogo".to_string());
        }

        match cmd {
            NetCmd::TogglePause => {
                self.game.running = !self.game.running;
                Ok(())
            }
            NetCmd::CycleSpeed => {
                self.cycle_speed();
                Ok(())
            }
            NetCmd::StartWave => {
                self.start_pending_wave();
                Ok(())
            }
            NetCmd::Build { x, y, kind } => {
                if self.build_at(x, y, kind) {
                    self.multiplayer.last_error = None;
                    Ok(())
                } else {
                    Err("nao foi possivel construir".to_string())
                }
            }
            NetCmd::Upgrade { x, y } => {
                let Some(idx) = self.tower_index_at(x, y) else {
                    return Err("sem torre para upgrade".to_string());
                };
                let cost = Self::tower_upgrade_cost(self.game.towers[idx].kind, self.game.wave);
                if !self.dev_mode && self.game.money < cost {
                    return Err("dinheiro insuficiente".to_string());
                }
                if self.game.towers[idx].level >= 9 {
                    return Err("torre no max".to_string());
                }
                if !self.dev_mode {
                    self.game.money -= cost;
                }
                self.game.towers[idx].level += 1;
                Ok(())
            }
            NetCmd::Sell { x, y } => {
                let Some(idx) = self.tower_index_at(x, y) else {
                    return Err("sem torre para vender".to_string());
                };
                self.game.towers.remove(idx);
                if !self.dev_mode {
                    self.game.money = self.game.money.saturating_add(20);
                }
                Ok(())
            }
            NetCmd::SetTargetMode { x, y, mode } => {
                let Some(idx) = self.tower_index_at(x, y) else {
                    return Err("sem torre para ajustar alvo".to_string());
                };
                self.game.towers[idx].target_mode = mode;
                Ok(())
            }
        }
    }

    fn snapshot_game(&self) -> GameSnapshot {
        GameSnapshot {
            running: self.game.running,
            speed: self.game.speed,
            money: self.game.money,
            lives: self.game.lives,
            wave: self.game.wave,
            pending_wave_start: self.game.pending_wave_start,
            prep_ticks: self.game.prep_ticks,
            towers: self.game.towers.clone(),
            enemies: self.game.enemies.clone(),
        }
    }

    fn apply_snapshot(&mut self, state: GameSnapshot) {
        let wave_before = self.game.wave;
        let selected_cell = self.game.selected_cell;
        let build_kind = self.game.build_kind;
        let pending_build = self.game.pending_build;

        self.game.running = state.running;
        self.game.speed = state.speed;
        self.game.money = state.money;
        self.game.lives = state.lives;
        self.game.wave = state.wave;
        self.game.pending_wave_start = state.pending_wave_start;
        self.game.prep_ticks = state.prep_ticks;
        self.game.towers = state.towers;
        self.game.enemies = state.enemies;

        self.game.selected_cell = selected_cell;
        self.game.build_kind = build_kind;
        self.game.pending_build = pending_build;

        if self.game.wave != wave_before {
            self.game.projectiles.clear();
            self.game.fx.clear();
        }
    }

    fn queue_fx_event(&mut self, event: FxEvent) {
        if !self.multiplayer.active || self.multiplayer.role != MultiplayerRole::Host {
            return;
        }
        self.multiplayer.pending_fx.push(event);
    }

    fn apply_fx_events(&mut self, events: Vec<FxEvent>) {
        if self.screen != Screen::Game {
            return;
        }
        if !self.multiplayer.active || self.multiplayer.status != ConnectionStatus::Connected {
            return;
        }

        for event in events {
            match event {
                FxEvent::Projectile {
                    kind,
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                    ttl,
                    seed,
                    muzzle_seed,
                    tracer_seed,
                } => {
                    let from = Vec2i::new(from_x as i16, from_y as i16);
                    let to = Vec2i::new(to_x as i16, to_y as i16);
                    let dir = Vec2i::new((to.x - from.x).signum(), (to.y - from.y).signum());

                    let ttl_fx = ttl.min(u8::MAX as u16) as u8;
                    let fx_id = self.game.fx.spawn_projectile(kind, from, to, ttl_fx, seed);
                    self.game.fx.spawn_muzzle(kind, from, dir, muzzle_seed);
                    if let Some(tracer_seed) = tracer_seed {
                        self.game.fx.spawn_tracer_line(kind, from, to, tracer_seed);
                    }

                    if let Some(fx_id) = fx_id {
                        self.game.projectiles.push(Projectile {
                            x: from.x,
                            y: from.y,
                            tx: to.x,
                            ty: to.y,
                            ttl,
                            damage: 0,
                            step_cd: Self::projectile_step_cd(kind),
                            kind,
                            source_level: 0,
                            fx_id: Some(fx_id),
                        });
                    }
                }
                FxEvent::TracerLine {
                    kind,
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                    seed,
                } => {
                    self.game.fx.spawn_tracer_line(
                        kind,
                        Vec2i::new(from_x as i16, from_y as i16),
                        Vec2i::new(to_x as i16, to_y as i16),
                        seed,
                    );
                }
                FxEvent::Impact { kind, x, y, seed } => {
                    let pos = Vec2i::new(x as i16, y as i16);
                    match kind {
                        TowerKind::Cannon => {
                            self.game.fx.spawn_impact_ring(pos, seed);
                            self.game.fx.spawn_dust(pos, seed ^ 0xA53C);
                        }
                        TowerKind::Tesla => {
                            self.game.fx.spawn_target_flash(pos, seed);
                        }
                        TowerKind::Frost => {
                            self.game.fx.spawn_shatter(pos, seed);
                        }
                        TowerKind::Sniper | TowerKind::Rapid | TowerKind::Basic => {
                            self.game.fx.spawn_impact_cross(pos, kind, seed);
                        }
                    }
                }
                FxEvent::ArcLightning {
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                    seed,
                } => {
                    self.game.fx.spawn_arc_lightning(
                        Vec2i::new(from_x as i16, from_y as i16),
                        Vec2i::new(to_x as i16, to_y as i16),
                        seed,
                    );
                }
                FxEvent::TargetFlash { x, y, seed } => {
                    self.game
                        .fx
                        .spawn_target_flash(Vec2i::new(x as i16, y as i16), seed);
                }
                FxEvent::Dust { x, y, seed } => {
                    self.game
                        .fx
                        .spawn_dust(Vec2i::new(x as i16, y as i16), seed);
                }
                FxEvent::Shatter { x, y, seed } => {
                    self.game
                        .fx
                        .spawn_shatter(Vec2i::new(x as i16, y as i16), seed);
                }
                FxEvent::StatusOverlay { target, ttl, seed } => {
                    self.game.fx.spawn_status_overlay(target, ttl, seed);
                }
            }
        }
    }

    fn send_multiplayer_cursor(&mut self) {
        if !self.multiplayer.active {
            return;
        }
        let (x, y) = self.game.selected_cell.unwrap_or((0, 0));
        let name = self
            .multiplayer
            .player_name
            .clone()
            .unwrap_or_else(|| self.multiplayer.name_input.clone())
            .trim()
            .to_string();
        let name = if name.is_empty() {
            "Player".to_string()
        } else {
            name
        };
        let pending_build = match (self.game.pending_build, self.game.build_kind) {
            (Some((bx, by)), Some(kind)) => Some((bx, by, kind)),
            _ => None,
        };
        if let Err(err) = self
            .multiplayer
            .queue_game_msg(&NetMsg::Cursor { name, x, y, pending_build })
        {
            self.multiplayer.status = ConnectionStatus::Failed;
            self.multiplayer.last_error = Some(err);
        }
    }

    fn send_multiplayer_state(&mut self) {
        if !self.multiplayer.active || self.multiplayer.role != MultiplayerRole::Host {
            return;
        }
        let snap = self.snapshot_game();
        if let Err(err) = self
            .multiplayer
            .queue_game_msg(&NetMsg::State { state: snap })
        {
            self.multiplayer.status = ConnectionStatus::Failed;
            self.multiplayer.last_error = Some(err);
        }
    }

    fn send_multiplayer_fx(&mut self) {
        if !self.multiplayer.active || self.multiplayer.role != MultiplayerRole::Host {
            self.multiplayer.pending_fx.clear();
            return;
        }

        if self.multiplayer.pending_fx.is_empty() {
            return;
        }

        let events = std::mem::take(&mut self.multiplayer.pending_fx);
        if let Err(err) = self.multiplayer.queue_game_msg(&NetMsg::Fx { events }) {
            self.multiplayer.status = ConnectionStatus::Failed;
            self.multiplayer.last_error = Some(err);
        }
    }

    fn parse_peer_addr(&self) -> Option<SocketAddr> {
        let peer = self.multiplayer.peer_ip.trim();
        if peer.is_empty() {
            return None;
        }
        if let Ok(addr) = peer.parse::<SocketAddr>() {
            return Some(addr);
        }
        None
    }

    fn ensure_cursor_slots(&mut self) {
        if !self.multiplayer.active {
            self.multiplayer.cursors.clear();
            return;
        }

        if self.multiplayer.cursors.is_empty() {
            let (x, y) = self.game.selected_cell.unwrap_or((0, 0));
            self.multiplayer.cursors.push(PlayerCursor {
                name: "Player".to_string(),
                x,
                y,
                pending_build: None,
            });
        }

        if self.multiplayer.cursors.len() == 1 {
            self.multiplayer.cursors.push(PlayerCursor {
                name: "Peer".to_string(),
                x: 0,
                y: 0,
                pending_build: None,
            });
        }

        if let Some(local) = self.multiplayer.cursors.first_mut() {
            if let Some((x, y)) = self.game.selected_cell {
                local.x = x;
                local.y = y;
            }
            if let Some(name) = self.multiplayer.player_name.as_ref() {
                if !name.trim().is_empty() {
                    local.name = name.clone();
                }
            }
        }

        if let Some(peer) = self.multiplayer.cursors.get_mut(1) {
            if let Some(name) = self.multiplayer.peer_name.as_ref() {
                if !name.trim().is_empty() {
                    peer.name = name.clone();
                }
            }
        }
    }

    fn update_multiplayer_cursors(&mut self) {
        if !self.multiplayer.active {
            return;
        }
        self.ensure_cursor_slots();
        if let Some(local) = self.multiplayer.cursors.first_mut() {
            if let Some((x, y)) = self.game.selected_cell {
                local.x = x;
                local.y = y;
            }
            if let Some(name) = self.multiplayer.player_name.as_ref() {
                local.name = name.clone();
            }
        }
    }

    pub fn multiplayer_cursors(&self) -> &[PlayerCursor] {
        self.multiplayer.cursors.as_slice()
    }

    fn reset_ui_for_game(&mut self) {
        self.ui.hover_cell = None;
        self.ui.hover_action = None;
        self.ui.hover_build_kind = None;
        self.ui.hover_button = None;
        self.ui.hover_map_select = None;
        self.ui.hover_multiplayer = None;
        self.ui.hover_main_menu = None;
        self.ui.hover_load_slot = None;
        self.ui.hover_load_wave = None;
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
                target_mode: t
                    .target_mode
                    .map(TargetMode::from)
                    .unwrap_or(TargetMode::Primeiro),
                fire_age: 0,
                fire_dir: (0, 0),
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
                target_mode: TargetMode::Primeiro,
                fire_age: 0,
                fire_dir: (0, 0),
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
            fx: FxManager::new(),
            pending_wave_start: false,
            prep_ticks: 0,
        };

        self.reset_ui_for_game();
        self.screen = Screen::Game;
        self.spawn_wave(false);
    }

    pub fn on_tick_if_due(&mut self) {
        if self.last_tick.elapsed() >= self.tick_rate {
            self.last_tick = Instant::now();
            self.ui.anim_tick = self.ui.anim_tick.wrapping_add(1);
            self.tick_top_notice();
            self.poll_network_events();
            self.tick_reconnect();

            if self.screen == Screen::Game {
                self.update_multiplayer_cursors();
                if !(self.multiplayer.active && self.is_multiplayer_peer()) {
                    self.on_tick();
                    self.send_multiplayer_state();
                    self.send_multiplayer_fx();
                } else {
                    self.tick_fx();
                    self.tick_projectiles_fx_only();
                }
                self.send_multiplayer_cursor();
            } else {
                self.tick_fx();
                self.send_multiplayer_cursor();
            }
        }
    }

    pub fn handle_button(&mut self, id: ButtonId) {
        if self.is_multiplayer_peer() {
            if self.multiplayer.status != ConnectionStatus::Connected {
                self.multiplayer.last_error = Some("sem conexao".to_string());
                return;
            }
            let cmd_option = match id {
                ButtonId::StartPause => Some(NetCmd::TogglePause),
                ButtonId::StartWave => Some(NetCmd::StartWave),
                ButtonId::Speed => Some(NetCmd::CycleSpeed),
                ButtonId::Quit => {
                    self.should_quit = true;
                    return;
                }
                _ => None,
            };
            if let Some(cmd_variant) = cmd_option {
                if let Err(err) = self.send_player_cmd(cmd_variant) {
                    self.multiplayer.status = ConnectionStatus::Failed;
                    self.multiplayer.last_error = Some(err);
                }
                return;
            }
        }

        match id {
            ButtonId::StartPause => self.game.running = !self.game.running,
            ButtonId::StartWave => self.start_pending_wave(),
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
        self.tick_enemy_rewards();
        self.tick_economy_and_waves();

        if !self.dev_mode && self.game.lives <= 0 {
            self.game.running = false;
        }
    }

    fn tick_fx(&mut self) {
        self.game.fx.tick();
    }

    fn tick_projectiles_fx_only(&mut self) {
        if !self.game.running {
            return;
        }

        let sp = self.game.speed.max(1) as u16;

        for p in &mut self.game.projectiles {
            if p.ttl > 0 {
                p.ttl -= 1;
            }
            if p.ttl == 0 {
                if let Some(fx_id) = p.fx_id {
                    self.game.fx.despawn(fx_id);
                }
                continue;
            }

            if p.step_cd > sp {
                p.step_cd -= sp;
                continue;
            }
            p.step_cd = Self::projectile_step_cd(p.kind);

            let dx = (p.tx - p.x).signum();
            let dy = (p.ty - p.y).signum();
            p.x += dx;
            p.y += dy;

            if let Some(fx_id) = p.fx_id {
                self.game
                    .fx
                    .update_projectile_pos(fx_id, Vec2i::new(p.x, p.y), Vec2i::new(dx, dy));
            }

            if p.x == p.tx && p.y == p.ty {
                p.ttl = 0;
                if let Some(fx_id) = p.fx_id {
                    self.game.fx.despawn(fx_id);
                }
            }
        }

        self.game.projectiles.retain(|p| p.ttl > 0);
    }

    fn tick_enemies(&mut self) {
        let sp = self.game.speed.max(1) as u16;
        let wave = self.game.wave;
        let mut healer_pulses: Vec<(u16, u16, u16, i32)> = Vec::new();

        for e in &mut self.game.enemies {
            if e.hp <= 0 {
                continue;
            }

            if e.slow_ticks > 0 {
                e.slow_ticks = e.slow_ticks.saturating_sub(1);
            } else {
                e.slow_percent = 0;
            }
            let tuning = Self::enemy_tuning(e.kind, wave);
            let effective_slow = Self::apply_slow_resist(e.slow_percent, tuning.slow_resist);
            let slow_factor = 100u16.saturating_sub(effective_slow as u16);
            let effective_sp = (sp * slow_factor / 100).max(1);

            if e.move_cd > effective_sp {
                e.move_cd -= effective_sp;
                continue;
            }

            // move 1 tile
            e.move_cd = tuning.move_cd;
            if e.path_i + 1 < self.game.path.len() {
                e.path_i += 1;
            } else {
                e.hp = 0;
                e.escaped = true;
                if !self.dev_mode {
                    self.game.lives = self.game.lives.saturating_sub(1);
                }
            }

            if e.hp > 0 && e.kind == EnemyKind::Healer && tuning.heal_amount > 0 {
                let (hx, hy) = self.game.path[e.path_i];
                healer_pulses.push((hx, hy, tuning.heal_radius, tuning.heal_amount));
            }
        }

        if healer_pulses.is_empty() {
            return;
        }

        for (hx, hy, radius, amount) in healer_pulses {
            for enemy in &mut self.game.enemies {
                if enemy.hp <= 0 {
                    continue;
                }
                let (ex, ey) = self.game.path[enemy.path_i];
                if manhattan(hx, hy, ex, ey) > radius {
                    continue;
                }
                let max_hp = Self::enemy_tuning(enemy.kind, wave).base_hp;
                enemy.hp = (enemy.hp + amount).min(max_hp);
            }
        }
    }

    fn tick_towers(&mut self) {
        let sp = self.game.speed.max(1) as u16;
        let wave = self.game.wave;
        let mut spawns: Vec<(u16, u16, u16, u16, i32, TowerKind, u8)> = Vec::new();

        let towers_len = self.game.towers.len();
        for ti in 0..towers_len {
            let mut tesla_action: Option<(u16, u16, u16, u16, i32, u8)> = None;
            {
                let t = &mut self.game.towers[ti];
                if t.fire_age > 0 {
                    t.fire_age = t.fire_age.saturating_sub(1);
                }
                let stats = Self::tower_stats(t);
                if t.kind == TowerKind::Tesla {
                    if t.cooldown > sp {
                        t.cooldown -= sp;
                    } else {
                        t.cooldown = 0;
                    }
                    let Some((tx, ty)) = Self::acquire_target(
                        t,
                        self.game.enemies.as_slice(),
                        self.game.path.as_slice(),
                        stats.range,
                        wave,
                    ) else {
                        continue;
                    };
                    let tick_damage = (stats.attack * sp as i32 / stats.fire_cd as i32).max(1);
                    if let Some(ei) = Self::enemy_index_at(
                        self.game.enemies.as_slice(),
                        self.game.path.as_slice(),
                        tx,
                        ty,
                    ) {
                        let e = &mut self.game.enemies[ei];
                        e.hp -= tick_damage;
                        if e.hp < 0 {
                            e.hp = 0;
                        }
                    }
                    t.fire_age = 4;
                    t.fire_dir = (
                        (tx as i16 - t.x as i16).signum(),
                        (ty as i16 - t.y as i16).signum(),
                    );
                    tesla_action = Some((t.x, t.y, tx, ty, tick_damage, t.level));
                } else {
                    if t.cooldown > sp {
                        t.cooldown -= sp;
                        continue;
                    }
                    t.cooldown = 0;

                    let Some((tx, ty)) = Self::acquire_target(
                        t,
                        self.game.enemies.as_slice(),
                        self.game.path.as_slice(),
                        stats.range,
                        wave,
                    ) else {
                        continue;
                    };

                    t.fire_dir = (
                        (tx as i16 - t.x as i16).signum(),
                        (ty as i16 - t.y as i16).signum(),
                    );
                    t.fire_age = 10;
                    spawns.push((t.x, t.y, tx, ty, stats.attack, t.kind, t.level));
                    t.cooldown = stats.fire_cd;
                }
            }
            if let Some((from_x, from_y, tx, ty, tick_damage, level)) = tesla_action {
                self.apply_tesla_chain(tx, ty, tick_damage, level);
                self.spawn_tesla_beam(from_x, from_y, tx, ty);
            }
        }

        for (from_x, from_y, to_x, to_y, dmg, kind, level) in spawns {
            self.spawn_projectile(from_x, from_y, to_x, to_y, dmg, kind, level);
        }
    }

    fn tick_projectiles(&mut self) {
        let sp = self.game.speed.max(1) as u16;
        let wave = self.game.wave;
        let mut impacts: Vec<(u16, u16, TowerKind, u8, Option<usize>)> = Vec::new();
        let mut on_hits: Vec<(TowerKind, u16, u16, i32, u8, Option<usize>)> = Vec::new();
        let mut status_overlays: Vec<(usize, u8)> = Vec::new();
        let mut hit_flashes: Vec<(u16, u16, EnemyKind)> = Vec::new();

        for p in &mut self.game.projectiles {
            if p.ttl > 0 {
                p.ttl -= 1;
            }
            if p.ttl == 0 {
                if let Some(fx_id) = p.fx_id {
                    self.game.fx.despawn(fx_id);
                }
                continue;
            }

            if p.step_cd > sp {
                p.step_cd -= sp;
                continue;
            }
            p.step_cd = Self::projectile_step_cd(p.kind);

            // move 1 passo em direção ao target (grid)
            let dx = (p.tx - p.x).signum();
            let dy = (p.ty - p.y).signum();
            p.x += dx;
            p.y += dy;

            if let Some(fx_id) = p.fx_id {
                self.game
                    .fx
                    .update_projectile_pos(fx_id, Vec2i::new(p.x, p.y), Vec2i::new(dx, dy));
            }

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
                    let tuning = Self::enemy_tuning(e.kind, wave);
                    let mut damage = p.damage;
                    if p.kind == TowerKind::Rapid {
                        damage = Self::apply_damage_resist(damage, tuning.rapid_resist);
                    }
                    e.hp -= damage;
                    if e.hp < 0 {
                        e.hp = 0;
                    }
                    if p.kind == TowerKind::Frost {
                        let (slow_percent, slow_ticks) = Self::frost_slow(p.source_level);
                        e.slow_percent = e.slow_percent.max(slow_percent);
                        e.slow_ticks = e.slow_ticks.max(slow_ticks);
                        status_overlays.push((ei, slow_ticks.min(u8::MAX as u16) as u8));
                    }
                    hit_flashes.push((hit_x, hit_y, e.kind));
                }
                on_hits.push((p.kind, hit_x, hit_y, p.damage, p.source_level, p.fx_id));

                impacts.push((hit_x, hit_y, p.kind, p.source_level, p.fx_id));
                p.ttl = 0;
            }
        }

        self.game.projectiles.retain(|p| p.ttl > 0);

        for (target, ttl) in status_overlays {
            let seed = self.rand_u32();
            self.game.fx.spawn_status_overlay(target, ttl, seed);
            self.queue_fx_event(FxEvent::StatusOverlay { target, ttl, seed });
        }

        for (hx, hy, ekind) in hit_flashes {
            let seed = self.rand_u32();
            self.game
                .fx
                .spawn_hit_flash(Vec2i::new(hx as i16, hy as i16), ekind, seed);
        }

        for (kind, hit_x, hit_y, damage, level, _fx_id) in on_hits {
            if kind == TowerKind::Tesla {
                self.apply_tesla_chain(hit_x, hit_y, damage, level);
            }
            if kind == TowerKind::Cannon {
                self.apply_cannon_splash(hit_x, hit_y, damage, level);
            }
            if kind == TowerKind::Frost {
                self.apply_frost_burst(hit_x, hit_y, level);
            }
        }

        for (x, y, kind, level, fx_id) in impacts {
            self.spawn_impact(x, y, kind, level);
            if let Some(fx_id) = fx_id {
                self.game.fx.despawn(fx_id);
            }
        }
    }

    fn tick_economy_and_waves(&mut self) {
        let alive = self.game.enemies.iter().any(|e| e.hp > 0);
        if !alive && (self.dev_mode || self.game.lives > 0) {
            if !self.game.pending_wave_start {
                self.game.wave += 1;
                self.game.pending_wave_start = true;
                self.game.prep_ticks = 0;
            } else {
                self.game.prep_ticks = self.game.prep_ticks.saturating_add(1);
            }
        }
    }

    fn spawn_wave(&mut self, append: bool) {
        // wave com mistura por budget (HP + velocidade + tipo).
        let wave = self.game.wave.max(1);
        let mut budget = 20 + wave * 12;
        let max_enemies = (9 + wave).clamp(9, 28) as usize;

        if !append {
            self.game.enemies.clear();
            self.game.projectiles.clear();
            self.game.fx.clear();
        }
        self.game.pending_wave_start = false;
        self.game.prep_ticks = 0;
        // stagger novo batch a partir dos inimigos já presentes
        let base_stagger = self.game.enemies.len() as u16 * 6;
        let mut spawned = 0usize;
        let mut attempts = 0;
        while budget > 0 && spawned < max_enemies && attempts < 100 {
            let mut kind = self.pick_enemy_kind(wave);
            let mut tuning = Self::enemy_tuning(kind, wave);
            let mut cost = Self::enemy_budget_cost(kind, tuning);
            if cost as i32 > budget {
                kind = EnemyKind::Swarm;
                tuning = Self::enemy_tuning(kind, wave);
                cost = Self::enemy_budget_cost(kind, tuning);
                if cost as i32 > budget {
                    break;
                }
            }

            let reward = Self::enemy_reward_value(cost, wave);
            let stagger = base_stagger + (spawned as u16 * 6).min(60);
            self.game.enemies.push(Enemy {
                kind,
                path_i: 0,
                hp: tuning.base_hp,
                move_cd: tuning.move_cd + stagger,
                slow_ticks: 0,
                slow_percent: 0,
                reward,
                rewarded: false,
                escaped: false,
            });
            budget -= cost as i32;
            spawned += 1;
            attempts += 1;
        }

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
                target_mode: Some(t.target_mode.into()),
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

    fn enemy_tuning(kind: EnemyKind, wave: i32) -> EnemyTuning {
        let base = Self::enemy_base_move_cd();
        let wave = wave.max(1);
        match kind {
            EnemyKind::Swarm => EnemyTuning {
                base_hp: 30 + wave * 9,
                move_cd: base + 2,
                slow_resist: 0,
                splash_resist: 0,
                rapid_resist: 0,
                heal_radius: 0,
                heal_amount: 0,
            },
            EnemyKind::Runner => EnemyTuning {
                base_hp: 24 + wave * 8,
                move_cd: base.saturating_sub(5).max(5),
                slow_resist: 0,
                splash_resist: 0,
                rapid_resist: 0,
                heal_radius: 0,
                heal_amount: 0,
            },
            EnemyKind::Tank => EnemyTuning {
                base_hp: 110 + wave * 28,
                move_cd: base + 3,
                slow_resist: 15,
                splash_resist: 30,
                rapid_resist: 15,
                heal_radius: 0,
                heal_amount: 0,
            },
            EnemyKind::Shielded => EnemyTuning {
                base_hp: 70 + wave * 18,
                move_cd: base + 1,
                slow_resist: 20,
                splash_resist: 15,
                rapid_resist: 45,
                heal_radius: 0,
                heal_amount: 0,
            },
            EnemyKind::Healer => EnemyTuning {
                base_hp: 60 + wave * 16,
                move_cd: base + 1,
                slow_resist: 10,
                splash_resist: 10,
                rapid_resist: 0,
                heal_radius: 1,
                heal_amount: 8 + wave,
            },
            EnemyKind::Sneak => EnemyTuning {
                base_hp: 40 + wave * 12,
                move_cd: base.saturating_sub(2).max(7),
                slow_resist: 20,
                splash_resist: 12,
                rapid_resist: 0,
                heal_radius: 0,
                heal_amount: 0,
            },
        }
    }

    fn enemy_budget_cost(kind: EnemyKind, tuning: EnemyTuning) -> i32 {
        let hp_cost = (tuning.base_hp / 25).max(1);
        let base_cd = Self::enemy_base_move_cd() as i32;
        let speed_cost = ((base_cd - tuning.move_cd as i32).max(0) / 2) + 1;
        let kind_cost = match kind {
            EnemyKind::Swarm => 0,
            EnemyKind::Runner => 1,
            EnemyKind::Sneak => 2,
            EnemyKind::Shielded => 3,
            EnemyKind::Healer => 4,
            EnemyKind::Tank => 5,
        };
        hp_cost + speed_cost + kind_cost
    }

    fn enemy_reward_value(cost: i32, wave: i32) -> i32 {
        let wave = wave.max(1);
        let base = cost.clamp(2, 14);
        let wave_bonus = (wave / 4).min(6);
        (base + wave_bonus).clamp(2, 20)
    }

    fn pick_enemy_kind(&mut self, wave: i32) -> EnemyKind {
        let wave = wave.max(1) as u32;
        let mut weights = vec![
            (EnemyKind::Swarm, 6 + wave / 2),
            (EnemyKind::Runner, 4 + wave / 3),
            (EnemyKind::Tank, 1 + wave / 5),
        ];
        if wave >= 5 {
            weights.push((EnemyKind::Healer, 2 + wave / 6));
        }
        if wave >= 8 {
            weights.push((EnemyKind::Shielded, 2 + wave / 7));
        }
        if wave >= 10 {
            weights.push((EnemyKind::Sneak, 2 + wave / 8));
        }
        let total: u32 = weights.iter().map(|(_, w)| *w).sum();
        let mut roll = self.rand_u32() % total;
        for (kind, weight) in weights {
            if roll < weight {
                return kind;
            }
            roll -= weight;
        }
        EnemyKind::Swarm
    }

    fn enemy_base_move_cd() -> u16 {
        // 50ms por tick
        // 14 ticks = 700ms por tile (bem mais lento)
        14
    }

    fn tick_enemy_rewards(&mut self) {
        let mut death_spawns: Vec<(EnemyKind, usize)> = Vec::new();
        for e in &mut self.game.enemies {
            if e.hp > 0 || e.rewarded || e.escaped {
                continue;
            }
            e.rewarded = true;
            if !self.dev_mode && e.reward > 0 {
                self.game.money = self.game.money.saturating_add(e.reward);
            }
            death_spawns.push((e.kind, e.path_i));
        }
        for (kind, path_i) in death_spawns {
            if let Some(&(px, py)) = self.game.path.get(path_i) {
                let seed = self.rand_u32();
                self.game.fx.spawn_enemy_death(
                    crate::fx::Vec2i::new(px as i16, py as i16),
                    kind,
                    seed,
                );
            }
        }
    }

    pub fn early_send_bonus(&self) -> i32 {
        let wave = self.game.wave.max(1);
        5 + wave * 2
    }

    fn start_pending_wave(&mut self) {
        let enemies_alive = self.game.enemies.iter().any(|e| e.hp > 0);
        if enemies_alive {
            // early send: próxima wave em cima das atuais → bônus por risco
            self.game.wave += 1;
            let bonus = self.early_send_bonus();
            if !self.dev_mode {
                self.game.money = self.game.money.saturating_add(bonus);
                self.show_top_notice(format!("+${bonus} early send"));
            }
            self.spawn_wave(true);
        } else if self.game.pending_wave_start {
            // envio normal entre waves
            self.spawn_wave(false);
        }
    }

    fn apply_slow_resist(slow_percent: u8, resist: u8) -> u8 {
        if slow_percent == 0 || resist == 0 {
            return slow_percent;
        }
        let reduction = (slow_percent as u16 * resist as u16 / 100) as u8;
        slow_percent.saturating_sub(reduction)
    }

    fn apply_damage_resist(damage: i32, resist: u8) -> i32 {
        if damage <= 0 || resist == 0 {
            return damage;
        }
        let reduction = (damage as i64 * resist as i64 / 100) as i32;
        (damage - reduction).max(0)
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

    fn spawn_tracer_line_event(
        &mut self,
        kind: TowerKind,
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
    ) {
        let from = Vec2i::new(from_x as i16, from_y as i16);
        let to = Vec2i::new(to_x as i16, to_y as i16);
        let seed = self.rand_u32();
        self.game.fx.spawn_tracer_line(kind, from, to, seed);
        self.queue_fx_event(FxEvent::TracerLine {
            kind,
            from_x,
            from_y,
            to_x,
            to_y,
            seed,
        });
    }

    fn spawn_tesla_beam(&mut self, from_x: u16, from_y: u16, to_x: u16, to_y: u16) {
        let from = Vec2i::new(from_x as i16, from_y as i16);
        let to   = Vec2i::new(to_x as i16, to_y as i16);
        let seed1 = self.rand_u32();
        let seed2 = self.rand_u32();
        self.game.fx.spawn_tesla_beam_fx(from, to, seed1, seed2);
        self.queue_fx_event(FxEvent::ArcLightning { from_x, from_y, to_x, to_y, seed: seed1 });
        self.queue_fx_event(FxEvent::ArcLightning { from_x, from_y, to_x, to_y, seed: seed2 });
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
        if kind == TowerKind::Tesla {
            self.spawn_tesla_beam(from_x, from_y, to_x, to_y);
            return;
        }
        if kind == TowerKind::Sniper {
            self.spawn_tracer_line_event(kind, from_x, from_y, to_x, to_y);
            if let Some(ei) = Self::enemy_index_at(
                self.game.enemies.as_slice(),
                self.game.path.as_slice(),
                to_x,
                to_y,
            ) {
                let e = &mut self.game.enemies[ei];
                e.hp -= dmg;
                if e.hp < 0 {
                    e.hp = 0;
                }
            }
            self.spawn_impact(to_x, to_y, kind, level);
            return;
        }
        let ttl = match kind {
            TowerKind::Sniper => 120,
            TowerKind::Cannon => 110,
            TowerKind::Tesla => 80,
            TowerKind::Frost => 90,
            TowerKind::Rapid => 70,
            TowerKind::Basic => 90,
        };
        let from = Vec2i::new(from_x as i16, from_y as i16);
        let to = Vec2i::new(to_x as i16, to_y as i16);
        let dir = Vec2i::new((to.x - from.x).signum(), (to.y - from.y).signum());

        let seed = self.rand_u32();
        let fx_id =
            self.game
                .fx
                .spawn_projectile(kind, from, to, ttl.min(u8::MAX as u16) as u8, seed);

        self.game.projectiles.push(Projectile {
            x: from.x,
            y: from.y,
            tx: to.x,
            ty: to.y,
            ttl,
            damage: dmg,
            step_cd: Self::projectile_step_cd(kind),
            kind,
            source_level: level,
            fx_id,
        });

        let muzzle_seed = self.rand_u32();
        self.game.fx.spawn_muzzle(kind, from, dir, muzzle_seed);
        let tracer_seed = None;

        self.queue_fx_event(FxEvent::Projectile {
            kind,
            from_x,
            from_y,
            to_x,
            to_y,
            ttl,
            seed,
            muzzle_seed,
            tracer_seed,
        });
    }

    fn spawn_impact(&mut self, x: u16, y: u16, kind: TowerKind, _level: u8) {
        let pos = Vec2i::new(x as i16, y as i16);
        let seed = self.rand_u32();
        match kind {
            TowerKind::Cannon => {
                self.game.fx.spawn_impact_ring(pos, seed);
                self.game.fx.spawn_dust(pos, seed ^ 0xA53C);
            }
            TowerKind::Tesla => {
                self.game.fx.spawn_target_flash(pos, seed);
            }
            TowerKind::Frost => {
                self.game.fx.spawn_shatter(pos, seed);
            }
            TowerKind::Sniper | TowerKind::Rapid | TowerKind::Basic => {
                self.game.fx.spawn_impact_cross(pos, kind, seed);
            }
        }

        self.queue_fx_event(FxEvent::Impact { kind, x, y, seed });
    }

    fn apply_tesla_chain(&mut self, x: u16, y: u16, damage: i32, level: u8) {
        if damage <= 0 {
            return;
        }
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

        let targets: Vec<(usize, u16, u16, u16)> =
            candidates.into_iter().take(max_targets).collect();
        let chain_bonus = 1.0 + (targets.len() as f32 * 0.1);

        for (idx, ex, ey, dist) in targets {
            let falloff = 1.0 - ((dist - 1) as f32 * 0.18).clamp(0.0, 0.6);
            let chain_damage =
                ((damage as f32) * (percent as f32 / 100.0) * falloff * chain_bonus).round() as i32;
            if chain_damage <= 0 {
                continue;
            }
            if let Some(e) = self.game.enemies.get_mut(idx) {
                e.hp -= chain_damage;
                if e.hp < 0 {
                    e.hp = 0;
                }
            }
            let arc_seed = self.rand_u32();
            let flash_seed = self.rand_u32();
            self.game.fx.spawn_arc_lightning(
                Vec2i::new(x as i16, y as i16),
                Vec2i::new(ex as i16, ey as i16),
                arc_seed,
            );
            self.game
                .fx
                .spawn_target_flash(Vec2i::new(ex as i16, ey as i16), flash_seed);

            self.queue_fx_event(FxEvent::ArcLightning {
                from_x: x,
                from_y: y,
                to_x: ex,
                to_y: ey,
                seed: arc_seed,
            });
            self.queue_fx_event(FxEvent::TargetFlash {
                x: ex,
                y: ey,
                seed: flash_seed,
            });
        }
    }

    fn apply_cannon_splash(&mut self, x: u16, y: u16, damage: i32, level: u8) {
        let (radius, percent) = Self::cannon_splash_params(level);
        let wave = self.game.wave;
        let mut fx_spawns: Vec<(i16, i16)> = Vec::new();
        for e in &mut self.game.enemies {
            if e.hp <= 0 {
                continue;
            }
            let (ex, ey) = self.game.path[e.path_i];
            let dist = manhattan(x, y, ex, ey);
            if dist == 0 || dist > radius {
                continue;
            }
            let tuning = Self::enemy_tuning(e.kind, wave);
            let splash_damage = ((damage as f32) * (percent as f32 / 100.0)).round() as i32;
            let splash_damage = Self::apply_damage_resist(splash_damage, tuning.splash_resist);
            if splash_damage <= 0 {
                continue;
            }
            e.hp -= splash_damage;
            if e.hp < 0 {
                e.hp = 0;
            }
            fx_spawns.push((ex as i16, ey as i16));
        }

        for (fx_x, fx_y) in fx_spawns {
            let pos = Vec2i::new(fx_x, fx_y);
            let seed = self.rand_u32();
            self.game.fx.spawn_dust(pos, seed);
            self.queue_fx_event(FxEvent::Dust {
                x: fx_x.max(0) as u16,
                y: fx_y.max(0) as u16,
                seed,
            });
        }
    }

    fn apply_frost_burst(&mut self, x: u16, y: u16, level: u8) {
        let (radius, slow_percent, slow_ticks) = Self::frost_burst_params(level);
        let mut fx_spawns: Vec<(usize, i16, i16)> = Vec::new();
        for (idx, e) in self.game.enemies.iter_mut().enumerate() {
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
            fx_spawns.push((idx, ex as i16, ey as i16));
        }

        for (idx, fx_x, fx_y) in fx_spawns {
            let pos = Vec2i::new(fx_x, fx_y);
            let shatter_seed = self.rand_u32();
            let overlay_seed = self.rand_u32();
            self.game.fx.spawn_shatter(pos, shatter_seed);
            self.game.fx.spawn_status_overlay(
                idx,
                slow_ticks.min(u8::MAX as u16) as u8,
                overlay_seed,
            );

            self.queue_fx_event(FxEvent::Shatter {
                x: fx_x.max(0) as u16,
                y: fx_y.max(0) as u16,
                seed: shatter_seed,
            });
            self.queue_fx_event(FxEvent::StatusOverlay {
                target: idx,
                ttl: slow_ticks.min(u8::MAX as u16) as u8,
                seed: overlay_seed,
            });
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

    fn acquire_target(
        tower: &Tower,
        enemies: &[Enemy],
        path: &[(u16, u16)],
        range: u16,
        wave: i32,
    ) -> Option<(u16, u16)> {
        let effective_range = Self::effective_tower_range(tower, enemies, path, range);
        let mut best: Option<(u16, u16, i32, usize, u16)> = None; // (ex, ey, score, path_i, dist)
        let path_len = path.len().max(1);
        for e in enemies {
            if e.hp <= 0 {
                continue;
            }
            let (ex, ey) = path[e.path_i];
            let dist = manhattan(tower.x, tower.y, ex, ey);
            if dist > effective_range {
                continue;
            }
            let score = Self::score_target(tower, e, path_len, dist, wave);
            match best {
                None => best = Some((ex, ey, score, e.path_i, dist)),
                Some((_, _, best_score, best_path_i, best_dist)) => {
                    if score > best_score
                        || (score == best_score
                            && (e.path_i > best_path_i
                                || (e.path_i == best_path_i && dist < best_dist)))
                    {
                        best = Some((ex, ey, score, e.path_i, dist));
                    }
                }
            }
        }
        best.map(|(ex, ey, _, _, _)| (ex, ey))
    }

    fn effective_tower_range(
        tower: &Tower,
        enemies: &[Enemy],
        path: &[(u16, u16)],
        range: u16,
    ) -> u16 {
        let sneak_radius = 2;
        let mut debuff = false;
        for e in enemies {
            if e.hp <= 0 || e.kind != EnemyKind::Sneak {
                continue;
            }
            let (ex, ey) = path[e.path_i];
            if manhattan(tower.x, tower.y, ex, ey) <= sneak_radius {
                debuff = true;
                break;
            }
        }
        if debuff && range > 1 {
            range - 1
        } else {
            range
        }
    }

    fn score_target(tower: &Tower, enemy: &Enemy, path_len: usize, dist: u16, wave: i32) -> i32 {
        let tuning = Self::enemy_tuning(enemy.kind, wave);
        let speed_score = (1000 / tuning.move_cd.max(1) as i32).max(1);
        let progress = enemy.path_i as i32;
        let path_len = path_len as i32;
        match tower.target_mode {
            TargetMode::Primeiro => progress * 1000 - dist as i32,
            TargetMode::Ultimo => (path_len - progress) * 1000 - dist as i32,
            TargetMode::MaisForte => enemy.hp,
            TargetMode::MaisFraco => -enemy.hp,
            TargetMode::MaisRapido => speed_score,
            TargetMode::MaisLento => -speed_score,
            TargetMode::MaisPerigoso => {
                let healer_bonus = if enemy.kind == EnemyKind::Healer {
                    120
                } else {
                    0
                };
                let shield_bonus = if enemy.kind == EnemyKind::Shielded {
                    60
                } else {
                    0
                };
                progress * 120 + enemy.hp / 4 + healer_bonus + shield_bonus - dist as i32
            }
            TargetMode::MaisCurador => {
                let healer_weight = if enemy.kind == EnemyKind::Healer {
                    2000
                } else {
                    0
                };
                healer_weight + progress * 50 - dist as i32
            }
        }
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

    pub fn enemy_kind_at(&self, x: u16, y: u16) -> Option<EnemyKind> {
        let idx = Self::enemy_index_at(
            self.game.enemies.as_slice(),
            self.game.path.as_slice(),
            x,
            y,
        )?;
        self.game.enemies.get(idx).map(|e| e.kind)
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

    pub fn tower_footprint(kind: TowerKind) -> (u16, u16) {
        match kind {
            TowerKind::Basic  => (1, 1),
            TowerKind::Sniper => (1, 1),
            TowerKind::Rapid  => (2, 1),
            TowerKind::Cannon => (2, 2),
            TowerKind::Tesla  => (1, 1),
            TowerKind::Frost  => (2, 2),
        }
    }

    pub fn tower_index_at(&self, x: u16, y: u16) -> Option<usize> {
        self.game.towers.iter().position(|t| {
            let (fw, fh) = Self::tower_footprint(t.kind);
            x >= t.x && x < t.x + fw && y >= t.y && y < t.y + fh
        })
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
        let next = (self.ui.zoom as i16 + delta).clamp(0, 4) as u16;
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
        if self.multiplayer.active && self.multiplayer.role == MultiplayerRole::Peer {
            return;
        }
        if self.map_index == 0 {
            self.map_index = self.maps.len() - 1;
        } else {
            self.map_index -= 1;
        }
        if self.multiplayer.active && self.multiplayer.role == MultiplayerRole::Host {
            if let Err(err) = self.multiplayer.queue_game_msg(&NetMsg::SetMap {
                map_index: self.map_index,
            }) {
                self.multiplayer.status = ConnectionStatus::Failed;
                self.multiplayer.last_error = Some(err);
            }
        }
    }

    pub fn select_next_map(&mut self) {
        if self.maps.is_empty() {
            return;
        }
        if self.multiplayer.active && self.multiplayer.role == MultiplayerRole::Peer {
            return;
        }
        self.map_index = (self.map_index + 1) % self.maps.len();
        if self.multiplayer.active && self.multiplayer.role == MultiplayerRole::Host {
            if let Err(err) = self.multiplayer.queue_game_msg(&NetMsg::SetMap {
                map_index: self.map_index,
            }) {
                self.multiplayer.status = ConnectionStatus::Failed;
                self.multiplayer.last_error = Some(err);
            }
        }
    }

    pub fn start_selected_map(&mut self) {
        if self.multiplayer.active && self.multiplayer.role == MultiplayerRole::Peer {
            self.multiplayer.last_error = Some("somente o host inicia a partida".to_string());
            self.multiplayer.last_info = None;
            return;
        }

        if self.multiplayer.active && self.multiplayer.role == MultiplayerRole::Host {
            if let Err(err) = self.multiplayer.queue_game_msg(&NetMsg::StartGame {
                map_index: self.map_index,
            }) {
                self.multiplayer.status = ConnectionStatus::Failed;
                self.multiplayer.last_error = Some(err);
            }
        }

        self.start_map_impl(self.map_index, true, true);

        if self.multiplayer.active && self.multiplayer.role == MultiplayerRole::Host {
            self.send_multiplayer_state();
        }
    }

    fn start_multiplayer_game_from_host(&mut self, map_index: usize) {
        if self.maps.is_empty() {
            return;
        }
        self.map_index = map_index.min(self.maps.len().saturating_sub(1));
        self.start_map_impl(self.map_index, false, false);
    }

    fn start_map_impl(&mut self, map_index: usize, create_save: bool, spawn_wave: bool) {
        if self.maps.is_empty() || map_index >= self.maps.len() {
            return;
        }
        let map = self.maps[map_index].clone();
        let selected_cell = Self::first_buildable(map.grid_w, map.grid_h, &map.path);

        self.game = GameState {
            running: false,
            speed: 1,
            money: 175,
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
                target_mode: TargetMode::Primeiro,
                fire_age: 0,
                fire_dir: (0, 0),
            }],
            enemies: vec![],
            projectiles: vec![],
            fx: FxManager::new(),
            pending_wave_start: false,
            prep_ticks: 0,
        };

        self.multiplayer.cursors.clear();
        self.ensure_cursor_slots();

        self.reset_ui_for_game();
        self.screen = Screen::Game;

        if create_save {
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
        } else {
            self.save_slot = None;
            self.last_save_error = None;
        }

        if spawn_wave {
            self.spawn_wave(false);
        }
    }

    fn is_multiplayer_peer(&self) -> bool {
        self.multiplayer.active && self.multiplayer.role == MultiplayerRole::Peer
    }

    fn send_player_cmd(&mut self, cmd: NetCmd) -> Result<(), String> {
        if self.multiplayer.status != ConnectionStatus::Connected {
            return Err("sem conexao".to_string());
        }
        let id = self.multiplayer.next_cmd_id;
        self.multiplayer.next_cmd_id = self.multiplayer.next_cmd_id.saturating_add(1);
        self.multiplayer.queue_game_msg(&NetMsg::Cmd { id, cmd })
    }

    fn request_build_at(&mut self, x: u16, y: u16, kind: TowerKind) {
        match self.send_player_cmd(NetCmd::Build { x, y, kind }) {
            Ok(()) => {
                self.multiplayer.last_error = None;
                self.multiplayer.last_info = None;
                self.game.build_kind = None;
                self.game.pending_build = None;
                self.game.selected_cell = Some((x, y));
            }
            Err(e) => {
                self.multiplayer.last_error = Some(e);
            }
        }
    }

    fn request_upgrade_at(&mut self, x: u16, y: u16) {
        match self.send_player_cmd(NetCmd::Upgrade { x, y }) {
            Ok(()) => {
                self.multiplayer.last_error = None;
                self.multiplayer.last_info = None;
            }
            Err(e) => self.multiplayer.last_error = Some(e),
        }
    }

    fn request_sell_at(&mut self, x: u16, y: u16) {
        match self.send_player_cmd(NetCmd::Sell { x, y }) {
            Ok(()) => {
                self.multiplayer.last_error = None;
                self.multiplayer.last_info = None;
            }
            Err(e) => self.multiplayer.last_error = Some(e),
        }
    }

    fn request_target_mode(&mut self, x: u16, y: u16, mode: TargetMode) {
        match self.send_player_cmd(NetCmd::SetTargetMode { x, y, mode }) {
            Ok(()) => {
                self.multiplayer.last_error = None;
                self.multiplayer.last_info = None;
            }
            Err(e) => self.multiplayer.last_error = Some(e),
        }
    }

    fn try_build(&mut self) {
        let Some(kind) = self.game.build_kind else {
            return;
        };
        let Some((x, y)) = self.game.selected_cell else {
            return;
        };
        self.game.pending_build = None;
        if self.is_multiplayer_peer() {
            self.request_build_at(x, y, kind);
            return;
        }
        if self.build_at(x, y, kind) {
            self.game.build_kind = None;
            self.game.selected_cell = Some((x, y));
        }
    }

    pub fn can_build_at(&self, x: u16, y: u16, kind: TowerKind) -> bool {
        let (fw, fh) = Self::tower_footprint(kind);
        for dy in 0..fh {
            for dx in 0..fw {
                let cx = x + dx;
                let cy = y + dy;
                if self.is_path(cx, cy) {
                    return false;
                }
                if self.game.towers.iter().any(|t| {
                    let (tw, th) = Self::tower_footprint(t.kind);
                    cx >= t.x && cx < t.x + tw && cy >= t.y && cy < t.y + th
                }) {
                    return false;
                }
            }
        }
        self.dev_mode || self.game.money >= Self::tower_cost(kind, self.game.wave)
    }

    pub fn build_at(&mut self, x: u16, y: u16, kind: TowerKind) -> bool {
        if !self.can_build_at(x, y, kind) {
            return false;
        }
        if !self.dev_mode {
            let cost = Self::tower_cost(kind, self.game.wave);
            self.game.money -= cost;
        }
        self.game.towers.push(Tower {
            x,
            y,
            kind,
            level: 1,
            cooldown: 0,
            target_mode: TargetMode::Primeiro,
            fire_age: 0,
            fire_dir: (0, 0),
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
        if self.is_multiplayer_peer() {
            self.request_upgrade_at(x, y);
            return;
        }

        let cost = Self::tower_upgrade_cost(self.game.towers[idx].kind, self.game.wave);
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
        if self.is_multiplayer_peer() {
            self.request_sell_at(x, y);
            return;
        }
        self.game.towers.remove(idx);
        if !self.dev_mode {
            self.game.money = self.game.money.saturating_add(20);
        }
    }

    pub fn cycle_selected_target_mode(&mut self) {
        let Some((x, y)) = self.game.selected_cell else {
            return;
        };
        let Some(idx) = self.tower_index_at(x, y) else {
            return;
        };
        let next = self.game.towers[idx].target_mode.next();
        if self.is_multiplayer_peer() {
            self.request_target_mode(x, y, next);
            return;
        }
        self.game.towers[idx].target_mode = next;
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

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut child = Command::new("clip")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("nao foi possivel copiar para a area de transferencia: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .map_err(|e| format!("falha ao copiar para a area de transferencia: {e}"))?;
        }

        let status = child
            .wait()
            .map_err(|e| format!("falha ao copiar para a area de transferencia: {e}"))?;
        if status.success() {
            return Ok(());
        }
        return Err("falha ao copiar para a area de transferencia".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = text;
        Err("area de transferencia nao suportada nesse sistema".to_string())
    }
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
                attack_step: 15,
                base_range: 6,
                range_every: 2,
                base_cd: 18,
                cd_drop_every: 4,
                cd_drop: 2,
                cd_min: 8,
                cd_max: 24,
            },
            TowerKind::Sniper => TowerTuning {
                base_attack: 90,
                attack_step: 22,
                base_range: 9,
                range_every: 3,
                base_cd: 26,
                cd_drop_every: 5,
                cd_drop: 2,
                cd_min: 14,
                cd_max: 30,
            },
            TowerKind::Rapid => TowerTuning {
                base_attack: 22,
                attack_step: 6,
                base_range: 5,
                range_every: 4,
                base_cd: 12,
                cd_drop_every: 3,
                cd_drop: 1,
                cd_min: 6,
                cd_max: 18,
            },
            TowerKind::Cannon => TowerTuning {
                base_attack: 120,
                attack_step: 30,
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
                attack_step: 13,
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
                attack_step: 9,
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

    pub fn tower_cost(kind: TowerKind, wave: i32) -> i32 {
        let base = match kind {
            TowerKind::Basic => 80,
            TowerKind::Sniper => 125,
            TowerKind::Rapid => 75,
            TowerKind::Cannon => 145,
            TowerKind::Tesla => 110,
            TowerKind::Frost => 100,
        };
        let wave = wave.max(1);
        let wave_bonus = (base * (wave - 1)) / 16;
        base + wave_bonus
    }

    pub fn tower_upgrade_cost(kind: TowerKind, wave: i32) -> i32 {
        let base = match kind {
            TowerKind::Basic => 65,
            TowerKind::Sniper => 90,
            TowerKind::Rapid => 58,
            TowerKind::Cannon => 100,
            TowerKind::Tesla => 75,
            TowerKind::Frost => 65,
        };
        let wave = wave.max(1);
        let wave_bonus = (base * (wave - 1)) / 18;
        base + wave_bonus
    }

    pub fn build_preview_stats(&self) -> Option<Stats> {
        let kind = self.game.build_kind?;
        let t = Tower {
            x: 0,
            y: 0,
            kind,
            level: 1,
            cooldown: 0,
            target_mode: TargetMode::Primeiro,
            fire_age: 0,
            fire_dir: (0, 0),
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
        // Mais “respirado” pra sprites grandes (4x4+).
        let grid_w = 60;
        let grid_h = 30;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 5), (grid_w - 2, 5));
        Self::push_segment(&mut path, (grid_w - 2, 5), (grid_w - 2, grid_h - 5));
        Self::push_segment(&mut path, (grid_w - 2, grid_h - 5), (2, grid_h - 5));
        MapSpec {
            name: "Serpentine",
            grid_w,
            grid_h,
            path,
        }
    }

    fn map_cascade() -> MapSpec {
        let grid_w = 72;
        let grid_h = 36;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 4), (grid_w - 3, 4));
        Self::push_segment(&mut path, (grid_w - 3, 4), (grid_w - 3, 16));
        Self::push_segment(&mut path, (grid_w - 3, 16), (4, 16));
        Self::push_segment(&mut path, (4, 16), (4, grid_h - 5));
        Self::push_segment(&mut path, (4, grid_h - 5), (grid_w - 2, grid_h - 5));
        MapSpec {
            name: "Cascade",
            grid_w,
            grid_h,
            path,
        }
    }

    fn map_spiral() -> MapSpec {
        let grid_w = 80;
        let grid_h = 40;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 6), (grid_w - 2, 6));
        Self::push_segment(&mut path, (grid_w - 2, 6), (grid_w - 2, grid_h - 6));
        Self::push_segment(&mut path, (grid_w - 2, grid_h - 6), (6, grid_h - 6));
        Self::push_segment(&mut path, (6, grid_h - 6), (6, 10));
        Self::push_segment(&mut path, (6, 10), (grid_w - 8, 10));
        Self::push_segment(&mut path, (grid_w - 8, 10), (grid_w - 8, grid_h - 10));
        Self::push_segment(&mut path, (grid_w - 8, grid_h - 10), (12, grid_h - 10));
        MapSpec {
            name: "Spiral",
            grid_w,
            grid_h,
            path,
        }
    }

    fn map_switchback() -> MapSpec {
        let grid_w = 70;
        let grid_h = 34;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 6), (grid_w - 2, 6));
        Self::push_segment(&mut path, (grid_w - 2, 6), (grid_w - 2, 12));
        Self::push_segment(&mut path, (grid_w - 2, 12), (3, 12));
        Self::push_segment(&mut path, (3, 12), (3, 18));
        Self::push_segment(&mut path, (3, 18), (grid_w - 4, 18));
        Self::push_segment(&mut path, (grid_w - 4, 18), (grid_w - 4, grid_h - 5));
        Self::push_segment(&mut path, (grid_w - 4, grid_h - 5), (8, grid_h - 5));
        MapSpec {
            name: "Switchback",
            grid_w,
            grid_h,
            path,
        }
    }

    fn map_crosswind() -> MapSpec {
        let grid_w = 74;
        let grid_h = 36;
        let mut path = Vec::new();
        Self::push_segment(&mut path, (1, 6), (grid_w - 2, 6));
        Self::push_segment(&mut path, (grid_w - 2, 6), (grid_w - 2, 12));
        Self::push_segment(&mut path, (grid_w - 2, 12), (6, 12));
        Self::push_segment(&mut path, (6, 12), (6, 24));
        Self::push_segment(&mut path, (6, 24), (grid_w - 6, 24));
        Self::push_segment(&mut path, (grid_w - 6, 24), (grid_w - 6, grid_h - 5));
        Self::push_segment(&mut path, (grid_w - 6, grid_h - 5), (3, grid_h - 5));
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
