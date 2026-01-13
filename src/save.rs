use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SAVE_DIR: &str = "saves";
const META_FILE: &str = "meta.json";
pub const SAVE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMeta {
    pub version: u32,
    pub id: String,
    pub created_at: u64,
    pub map_name: String,
    pub dev_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveCheckpoint {
    pub version: u32,
    pub saved_at: u64,
    pub map_name: String,
    pub dev_mode: bool,
    pub wave: i32,
    pub money: i32,
    pub lives: i32,
    pub speed: u8,
    pub towers: Vec<SaveTower>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveTower {
    pub x: u16,
    pub y: u16,
    pub kind: TowerKindSave,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TowerKindSave {
    Basic,
    Sniper,
    Rapid,
    Cannon,
    Tesla,
    Frost,
}

#[derive(Debug, Clone)]
pub struct SaveSlotSummary {
    pub id: String,
    pub created_at: u64,
    pub map_name: String,
    pub dev_mode: bool,
    pub waves: Vec<i32>,
}

pub fn create_new_slot(map_name: &str, dev_mode: bool) -> Result<String> {
    ensure_root_dir()?;

    let id = new_slot_id(map_name);
    let slot_dir = slot_dir(&id);
    fs::create_dir_all(&slot_dir)
        .with_context(|| format!("failed to create save slot dir: {}", slot_dir.display()))?;

    let meta = SaveMeta {
        version: SAVE_VERSION,
        id: id.clone(),
        created_at: unix_secs(),
        map_name: map_name.to_string(),
        dev_mode,
    };
    write_json_atomic(&slot_dir.join(META_FILE), &meta)?;

    Ok(id)
}

pub fn write_wave_checkpoint(slot_id: &str, checkpoint: &SaveCheckpoint) -> Result<()> {
    ensure_root_dir()?;
    let slot_dir = slot_dir(slot_id);
    fs::create_dir_all(&slot_dir)
        .with_context(|| format!("failed to create save slot dir: {}", slot_dir.display()))?;

    let path = slot_dir.join(wave_filename(checkpoint.wave));
    write_json_atomic(&path, checkpoint)
        .with_context(|| format!("failed to write checkpoint: {}", path.display()))?;
    Ok(())
}

pub fn list_slots() -> Result<Vec<SaveSlotSummary>> {
    let root = PathBuf::from(SAVE_DIR);
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut slots = Vec::new();
    for entry in
        fs::read_dir(&root).with_context(|| format!("failed to read {}", root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = entry
            .file_name()
            .into_string()
            .unwrap_or_else(|_| "<invalid>".to_string());

        let meta_path = path.join(META_FILE);
        let meta: SaveMeta = match read_json(&meta_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let mut waves = Vec::new();
        if let Ok(files) = fs::read_dir(&path) {
            for f in files.flatten() {
                let p = f.path();
                if p.extension() != Some(OsStr::new("json")) {
                    continue;
                }
                let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                if let Some(w) = parse_wave_stem(stem) {
                    waves.push(w);
                }
            }
        }
        waves.sort_unstable();
        waves.dedup();

        slots.push(SaveSlotSummary {
            id,
            created_at: meta.created_at,
            map_name: meta.map_name,
            dev_mode: meta.dev_mode,
            waves,
        });
    }

    slots.sort_by_key(|s| std::cmp::Reverse(s.created_at));
    Ok(slots)
}

pub fn load_checkpoint(slot_id: &str, wave: i32) -> Result<SaveCheckpoint> {
    let path = slot_dir(slot_id).join(wave_filename(wave));
    read_json(&path).with_context(|| format!("failed to read checkpoint: {}", path.display()))
}

fn ensure_root_dir() -> Result<()> {
    fs::create_dir_all(SAVE_DIR)
        .with_context(|| format!("failed to create save root dir: {}", SAVE_DIR))?;
    Ok(())
}

fn slot_dir(slot_id: &str) -> PathBuf {
    PathBuf::from(SAVE_DIR).join(slot_id)
}

fn wave_filename(wave: i32) -> String {
    // padding ajuda na ordena‡Æo por nome no filesystem
    format!("wave_{:04}.json", wave.max(0))
}

fn parse_wave_stem(stem: &str) -> Option<i32> {
    // wave_0001
    let wave = stem.strip_prefix("wave_")?;
    wave.parse().ok()
}

fn new_slot_id(map_name: &str) -> String {
    let safe_map = sanitize(map_name);
    let nanos = unix_nanos();
    format!("{safe_map}-{nanos}")
}

fn sanitize(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '_' {
            out.push('_');
        }
    }
    if out.is_empty() {
        "save".to_string()
    } else {
        out
    }
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let txt = fs::read_to_string(path)?;
    serde_json::from_str(&txt).with_context(|| format!("invalid json: {}", path.display()))
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let txt = serde_json::to_string_pretty(value)?;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let tmp = path.with_extension("tmp");
    fs::write(&tmp, txt.as_bytes())?;
    if let Err(e) = fs::rename(&tmp, path) {
        // Windows não sobrescreve no rename; garante overwrite do checkpoint.
        if path.exists() {
            let _ = fs::remove_file(path);
            fs::rename(&tmp, path)?;
        } else {
            return Err(e.into());
        }
    }
    Ok(())
}
