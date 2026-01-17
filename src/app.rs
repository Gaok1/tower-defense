use crate::{
    fx::{FxManager, Vec2i},
    net_msg::{GameSnapshot, NetCmd, NetMsg},
    p2p_connection::{AutotuneConfig, NetCommand, NetEvent, start_network},
    save,
};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, to_vec};
use std::{
    io::Write,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc::UnboundedSender;

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
    pub network: Option<MultiplayerNetwork>,
    pub next_cmd_id: u32,
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
            network: None,
            next_cmd_id: 1,
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
        if let Some(net) = self.network.as_mut() {
            if net.bind_addr == bind_addr {
                return;
            }
            let _ = net.cmd_tx.send(NetCommand::Rebind(bind_addr));
            net.bind_addr = bind_addr;
            return;
        }

        let autotune = AutotuneConfig::default();
        let (cmd_tx, evt_rx, handle) = start_network(bind_addr, None, autotune);
        self.network = Some(MultiplayerNetwork {
            cmd_tx,
            evt_rx,
            handle,
            bind_addr,
        });
    }

    fn shutdown_network(&mut self) {
        if let Some(network) = self.network.take() {
            let _ = network.cmd_tx.send(NetCommand::Shutdown);
            let _ = network.handle.join();
        }
    }

    fn send_net_command(&mut self, cmd: NetCommand) -> Result<(), String> {
        if let Some(net) = self.network.as_mut() {
            net.cmd_tx
                .send(cmd)
                .map_err(|_| "conexao multiplayer encerrada".to_string())
        } else {
            Err("rede multiplayer inativa".to_string())
        }
    }

    fn queue_game_msg(&mut self, msg: &NetMsg) -> Result<(), String> {
        let payload = to_vec(msg).map_err(|e| e.to_string())?;
        self.send_net_command(NetCommand::GameMessage(payload))
    }

    fn refresh_network(&mut self) {
        let bind_addr = self.current_bind_addr();
        if let Some(net) = self.network.as_mut() {
            net.bind_addr = bind_addr;
            let _ = net.cmd_tx.send(NetCommand::Rebind(bind_addr));
        } else {
            self.ensure_network();
        }
    }
}

#[derive(Debug)]
struct MultiplayerNetwork {
    cmd_tx: UnboundedSender<NetCommand>,
    evt_rx: mpsc::Receiver<NetEvent>,
    handle: thread::JoinHandle<()>,
    bind_addr: SocketAddr,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TowerKind {
    Basic,
    Sniper,
    Rapid,
    Cannon,
    Tesla,
    Frost,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tower {
    pub x: u16,
    pub y: u16,
    pub kind: TowerKind,
    pub level: u8,
    pub cooldown: u16, // ticks até poder atirar
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
                hit: UiHitboxes::default(),
                viewport: MapViewport::default(),
                zoom: 1,
                last_zoom: 1, // <-- NOVO
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
                fx: FxManager::new(),
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
        self.multiplayer.shutdown_network();
        self.multiplayer.cursors.clear();
    }

    pub fn enter_load_game(&mut self) {
        self.multiplayer.active = false;
        self.multiplayer.shutdown_network();
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
        self.multiplayer.role = match self.multiplayer.role {
            MultiplayerRole::Host => MultiplayerRole::Peer,
            MultiplayerRole::Peer => MultiplayerRole::Host,
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
            self.multiplayer.last_error = Some("IP publico ainda nao detectado".to_string());
            return;
        };

        let endpoint_text = endpoint.to_string();
        match copy_to_clipboard(&endpoint_text) {
            Ok(()) => {
                self.multiplayer.last_info = Some("IP copiado para o clipboard".to_string());
            }
            Err(e) => {
                self.multiplayer.last_error = Some(format!("falha ao copiar IP: {e}"));
            }
        }
    }

    pub fn multiplayer_connect(&mut self) {
        self.multiplayer.active = false;
        self.multiplayer.last_error = None;
        self.multiplayer.last_info = None;
        self.multiplayer.ensure_network();

        match self.multiplayer.role {
            MultiplayerRole::Host => {
                self.multiplayer.status = ConnectionStatus::Connecting;
                let info = self
                    .multiplayer
                    .local_endpoint
                    .map(|addr| format!("aguardando conexao em {addr}"))
                    .unwrap_or_else(|| "aguardando conexao".to_string());
                self.multiplayer.last_info = Some(info);
            }
            MultiplayerRole::Peer => {
                if self.multiplayer.peer_ip.trim().is_empty() {
                    self.multiplayer.status = ConnectionStatus::Failed;
                    self.multiplayer.last_error = Some("IP do host vazio".to_string());
                    return;
                }

                let Some(addr) = self.parse_peer_addr() else {
                    self.multiplayer.status = ConnectionStatus::Failed;
                    self.multiplayer.last_error = Some("IP do host invalido".to_string());
                    return;
                };

                if let Err(err) = self
                    .multiplayer
                    .send_net_command(NetCommand::ConnectPeer(addr))
                {
                    self.multiplayer.status = ConnectionStatus::Failed;
                    self.multiplayer.last_error = Some(err);
                    return;
                }
                self.multiplayer.status = ConnectionStatus::Connecting;
                self.multiplayer.last_info = Some(format!("conectando em {addr}"));
            }
        }
    }

    pub fn multiplayer_continue(&mut self) {
        if self.multiplayer.status != ConnectionStatus::Connected {
            return;
        }
        if self.multiplayer.name_input.trim().is_empty() {
            self.multiplayer.last_error = Some("defina o nome do player".to_string());
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
        self.screen = Screen::MapSelect;
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
                        self.multiplayer.last_error =
                            Some("canal de rede encerrado".to_string());
                        self.multiplayer.network = None;
                        return;
                    }
                }
            };

            self.handle_net_event(event);
        }
    }

    fn handle_net_event(&mut self, event: NetEvent) {
        match event {
            NetEvent::Bound(addr) => {
                self.multiplayer.last_info = Some(format!("ouvindo {addr}"));
            }
            NetEvent::PublicEndpoint(addr) => {
                self.multiplayer.local_endpoint = Some(addr);
                if self.multiplayer.status == ConnectionStatus::FetchingIp {
                    self.multiplayer.status = ConnectionStatus::Ready;
                    self.multiplayer.last_info = Some(format!("IP publico {addr}"));
                }
            }
            NetEvent::PeerConnecting(peer) => {
                self.multiplayer.last_info = Some(format!("conectando em {peer}"));
            }
            NetEvent::PeerConnected(peer) => {
                self.multiplayer.status = ConnectionStatus::Connected;
                self.multiplayer.active = true;
                self.multiplayer.last_error = None;
                self.multiplayer.last_info = Some(format!("peer conectado: {peer}"));
                self.ensure_cursor_slots();

                if self.multiplayer.role == MultiplayerRole::Host {
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
            NetEvent::PeerDisconnected(peer) => {
                self.multiplayer.status = ConnectionStatus::Failed;
                self.multiplayer.active = false;
                self.multiplayer.last_error = Some(format!("peer desconectado: {peer}"));
            }
            NetEvent::PeerTimeout(peer) => {
                self.multiplayer.status = ConnectionStatus::Failed;
                self.multiplayer.active = false;
                self.multiplayer.last_error = Some(format!("tempo esgotado {peer}"));
            }
            NetEvent::PublicEndpointObserved(addr) => {
                self.multiplayer.last_info = Some(format!("endpoint observado {addr}"));
            }
            NetEvent::Log(msg) => {
                if msg.to_lowercase().contains("erro") {
                    self.multiplayer.last_error = Some(msg);
                    if self.multiplayer.status == ConnectionStatus::FetchingIp {
                        self.multiplayer.status = ConnectionStatus::Failed;
                    }
                } else {
                    self.multiplayer.last_info = Some(msg);
                }
            }
            NetEvent::GameMessage(msg) => match from_slice(&msg) {
                Ok(net_msg) => self.handle_net_msg(net_msg),
                Err(err) => {
                    self.multiplayer.last_error =
                        Some(format!("falha ao decodificar mensagem: {err}"));
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
                match self.multiplayer.role {
                    MultiplayerRole::Host => {
                        self.multiplayer.peer_name = Some(name);
                    }
                    MultiplayerRole::Peer => {
                        self.multiplayer.peer_name = Some(name);
                    }
                }
                self.ensure_cursor_slots();
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
            NetMsg::Cursor { name, x, y } => {
                if !self.multiplayer.active {
                    return;
                }
                self.ensure_cursor_slots();
                if self.multiplayer.role == MultiplayerRole::Host {
                    if let Some(peer) = self.multiplayer.cursors.get_mut(1) {
                        peer.name = name;
                        peer.x = x;
                        peer.y = y;
                    }
                } else if let Some(peer) = self.multiplayer.cursors.get_mut(1) {
                    peer.name = name;
                    peer.x = x;
                    peer.y = y;
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
                if ok {
                    self.multiplayer.last_error = None;
                } else {
                    self.multiplayer.last_error =
                        Some(error.unwrap_or_else(|| "falha".to_string()));
                }
            }
            NetMsg::State { state } => {
                if self.multiplayer.role != MultiplayerRole::Peer {
                    return;
                }
                self.apply_snapshot(state);
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
                let cost = Self::tower_upgrade_cost(self.game.towers[idx].kind);
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
        }
    }

    fn snapshot_game(&self) -> GameSnapshot {
        GameSnapshot {
            running: self.game.running,
            speed: self.game.speed,
            money: self.game.money,
            lives: self.game.lives,
            wave: self.game.wave,
            towers: self.game.towers.clone(),
            enemies: self.game.enemies.clone(),
        }
    }

    fn apply_snapshot(&mut self, state: GameSnapshot) {
        let selected_cell = self.game.selected_cell;
        let build_kind = self.game.build_kind;
        let pending_build = self.game.pending_build;

        self.game.running = state.running;
        self.game.speed = state.speed;
        self.game.money = state.money;
        self.game.lives = state.lives;
        self.game.wave = state.wave;
        self.game.towers = state.towers;
        self.game.enemies = state.enemies;

        self.game.selected_cell = selected_cell;
        self.game.build_kind = build_kind;
        self.game.pending_build = pending_build;
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
        if let Err(err) = self
            .multiplayer
            .queue_game_msg(&NetMsg::Cursor { name, x, y })
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
            });
        }

        if self.multiplayer.cursors.len() == 1 {
            self.multiplayer.cursors.push(PlayerCursor {
                name: "Peer".to_string(),
                x: 0,
                y: 0,
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
            fx: FxManager::new(),
            money_cd: 0,
        };

        self.reset_ui_for_game();
        self.screen = Screen::Game;
        self.spawn_wave();
    }

    pub fn on_tick_if_due(&mut self) {
        if self.last_tick.elapsed() >= self.tick_rate {
            self.last_tick = Instant::now();
            self.poll_network_events();
            if self.screen == Screen::Game {
                self.update_multiplayer_cursors();
                if !(self.multiplayer.active && self.is_multiplayer_peer()) {
                    self.on_tick();
                    self.send_multiplayer_state();
                } else {
                    self.tick_fx();
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
        self.game.fx.tick();
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
        let mut impacts: Vec<(u16, u16, TowerKind, u8, Option<usize>)> = Vec::new();
        let mut on_hits: Vec<(TowerKind, u16, u16, i32, u8, Option<usize>)> = Vec::new();
        let mut status_overlays: Vec<(usize, u8)> = Vec::new();

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
                    e.hp -= p.damage;
                    if e.hp < 0 {
                        e.hp = 0;
                    }
                    if p.kind == TowerKind::Frost {
                        let (slow_percent, slow_ticks) = Self::frost_slow(p.source_level);
                        e.slow_percent = e.slow_percent.max(slow_percent);
                        e.slow_ticks = e.slow_ticks.max(slow_ticks);
                        status_overlays.push((ei, slow_ticks.min(u8::MAX as u16) as u8));
                    }
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
        self.game.fx.clear();

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
        _level: u8,
    ) {
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
            source_level: _level,
            fx_id,
        });

        let muzzle_seed = self.rand_u32();
        self.game.fx.spawn_muzzle(kind, from, dir, muzzle_seed);
        if kind == TowerKind::Sniper {
            let tracer_seed = self.rand_u32();
            self.game.fx.spawn_tracer_line(from, to, tracer_seed);
        }
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
        }
    }

    fn apply_cannon_splash(&mut self, x: u16, y: u16, damage: i32, level: u8) {
        let (radius, percent) = Self::cannon_splash_params(level);
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
            let splash_damage = ((damage as f32) * (percent as f32 / 100.0)).round() as i32;
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
            fx: FxManager::new(),
            money_cd: 0,
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
            self.spawn_wave();
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
        if self.is_multiplayer_peer() {
            self.request_upgrade_at(x, y);
            return;
        }

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
        if self.is_multiplayer_peer() {
            self.request_sell_at(x, y);
            return;
        }
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

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut child = Command::new("clip")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("nao foi possivel executar `clip`: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .map_err(|e| format!("falha ao escrever no clipboard: {e}"))?;
        }

        let status = child
            .wait()
            .map_err(|e| format!("falha ao aguardar `clip`: {e}"))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("`clip` falhou: {status:?}"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = text;
        Err("clipboard nao suportado nesse sistema".to_string())
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
