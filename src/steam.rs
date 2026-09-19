use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::PopupProfile;
use super::config;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) enum SelectionMode {
    All,
    Selected,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum ItemKind {
    Game,
    Other,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Settings {
    pub(super) enabled: bool,
    pub(super) manual_libraries: Vec<PathBuf>,
    pub(super) selection_mode: SelectionMode,
    pub(super) selected_games: HashSet<u32>,
    pub(super) use_custom_profile: bool,
    pub(super) common_profile: PopupProfile,
    pub(super) game_profiles: HashMap<u32, PopupProfile>,
    pub(super) manual_executables: HashMap<u32, Vec<PathBuf>>,
    pub(super) show_fps: bool,
    pub(super) show_frame_time: bool,
    #[serde(default)]
    pub(super) include_other: bool,
}

impl Settings {
    pub(super) fn new(profile: PopupProfile) -> Self {
        Self {
            enabled: false,
            manual_libraries: Vec::new(),
            selection_mode: SelectionMode::All,
            selected_games: HashSet::new(),
            use_custom_profile: false,
            common_profile: profile,
            game_profiles: HashMap::new(),
            manual_executables: HashMap::new(),
            show_fps: false,
            show_frame_time: false,
            include_other: false,
        }
    }

    pub(super) fn sanitize(&mut self) {
        self.common_profile.sanitize();
        for profile in self.game_profiles.values_mut() {
            profile.sanitize();
        }
        self.manual_libraries.sort();
        self.manual_libraries.dedup();
    }
}

fn settings_file() -> PathBuf {
    config::settings_file().with_file_name("steam-integration.json")
}

pub(super) fn encode(settings: &Settings) -> Result<String, String> {
    serde_json::to_string_pretty(settings).map_err(|error| error.to_string())
}

pub(super) fn load(profile: PopupProfile) -> Settings {
    let Ok(value) = std::fs::read_to_string(settings_file()) else {
        return Settings::new(profile);
    };
    let Ok(mut settings) = serde_json::from_str::<Settings>(&value) else {
        return Settings::new(profile);
    };
    settings.sanitize();
    settings
}

pub(super) fn save(value: &str) -> Result<(), String> {
    let path = settings_file();
    let parent = path
        .parent()
        .ok_or_else(|| "Steam 설정 경로를 만들 수 없습니다.".to_owned())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(path, value).map_err(|error| error.to_string())
}

#[derive(Clone)]
pub(super) struct Game {
    pub(super) app_id: u32,
    pub(super) name: String,
    pub(super) install_dir: PathBuf,
    pub(super) icon: Option<Vec<u8>>,
    pub(super) kind: ItemKind,
}

pub(super) struct ScanResult {
    pub(super) games: Vec<Game>,
    pub(super) libraries: Vec<PathBuf>,
    pub(super) message: String,
}

pub(super) fn manifest_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut quoted = line.split('"').skip(1).step_by(2);
        let found_key = quoted.next()?;
        let value = quoted.next()?;
        found_key
            .eq_ignore_ascii_case(key)
            .then(|| value.replace(r"\\", r"\"))
    })
}

pub(super) fn fallback_kind(name: &str) -> ItemKind {
    let name = name.to_ascii_lowercase();
    let other_markers = [
        "steamworks common redistributables",
        "steam linux runtime",
        "proton ",
        "wallpaper engine",
        "lossless scaling",
        "dedicated server",
        "sdk",
        "benchmark tool",
    ];
    if other_markers.iter().any(|marker| name.contains(marker)) {
        ItemKind::Other
    } else {
        ItemKind::Game
    }
}

pub(super) fn kind_from_type(value: &str, name: &str) -> ItemKind {
    if fallback_kind(name) == ItemKind::Other {
        return ItemKind::Other;
    }
    match value.to_ascii_lowercase().as_str() {
        "game" | "dlc" | "demo" | "mod" => ItemKind::Game,
        "application" | "tool" | "video" | "music" | "hardware" | "series" => ItemKind::Other,
        _ => fallback_kind(name),
    }
}

pub(super) fn normalize_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{ItemKind, fallback_kind, kind_from_type, manifest_value};

    #[test]
    fn manifest_values_are_parsed() {
        let manifest = r#""AppState"
        {
            "appid" "570"
            "name" "Dota 2"
            "installdir" "dota 2 beta"
        }"#;
        assert_eq!(manifest_value(manifest, "appid").as_deref(), Some("570"));
        assert_eq!(manifest_value(manifest, "name").as_deref(), Some("Dota 2"));
        assert_eq!(
            manifest_value(manifest, "installdir").as_deref(),
            Some("dota 2 beta")
        );
    }

    #[test]
    fn games_and_other_items_are_classified() {
        assert!(matches!(kind_from_type("game", "Example"), ItemKind::Game));
        assert!(matches!(kind_from_type("dlc", "Expansion"), ItemKind::Game));
        assert!(matches!(kind_from_type("demo", "Demo"), ItemKind::Game));
        assert!(matches!(
            kind_from_type("application", "Wallpaper Engine"),
            ItemKind::Other
        ));
        assert!(matches!(
            fallback_kind("Steamworks Common Redistributables"),
            ItemKind::Other
        ));
        assert!(matches!(fallback_kind("Proton 9.0"), ItemKind::Other));
        assert!(matches!(
            kind_from_type("game", "Wallpaper Engine"),
            ItemKind::Other
        ));
        assert!(matches!(
            kind_from_type("game", "Lossless Scaling"),
            ItemKind::Other
        ));
    }
}
