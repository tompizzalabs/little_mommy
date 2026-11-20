use crate::engine::graph::Patch;
use crate::presets;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const PRESET_EXTENSION: &str = "lmp";
pub const PRESET_FORMAT_VERSION: u32 = 1;
const FORMAT_VERSION: u32 = PRESET_FORMAT_VERSION;
const COMMUNITY_DIR: &str = "Community";
const USER_DIR: &str = "User";
const FALLBACK_CATEGORY: &str = "Misc";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetFile {
    pub format: u32,
    pub name: String,
    pub author: String,
    pub category: String,
    pub patch: Patch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetOrigin {
    Community,
    User,
}

impl PresetOrigin {
    pub fn label(self) -> &'static str {
        match self {
            PresetOrigin::Community => "Community",
            PresetOrigin::User => "User",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PresetEntry {
    pub name: String,
    pub author: String,
    pub category: String,
    pub origin: PresetOrigin,
    pub path: PathBuf,
}

pub fn preset_root() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    return std::env::var_os("APPDATA")
        .map(|base| PathBuf::from(base).join("Sozvezdie").join("Presets"));

    #[cfg(target_os = "macos")]
    return std::env::var_os("HOME").map(|base| {
        PathBuf::from(base)
            .join("Library")
            .join("Application Support")
            .join("Sozvezdie")
            .join("Presets")
    });

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map(|base| base.join("sozvezdie").join("Presets"));
}

pub fn sanitize_file_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '-' | '_' | '.' => character,
            _ => '_',
        })
        .collect();
    cleaned.trim().to_string()
}

pub fn install_community_presets() -> io::Result<()> {
    let Some(root) = preset_root() else {
        return Ok(());
    };
    let community = root.join(COMMUNITY_DIR);
    let mut installed_paths: Vec<PathBuf> = Vec::new();
    for preset in presets::factory_presets() {
        let category_dir = community.join(preset.category);
        fs::create_dir_all(&category_dir)?;
        let path = category_dir.join(format!(
            "{}.{PRESET_EXTENSION}",
            sanitize_file_name(preset.name)
        ));
        if path.exists() {
            installed_paths.push(path);
            continue;
        }
        let file = PresetFile {
            format: FORMAT_VERSION,
            name: preset.name.to_owned(),
            author: preset.author.to_owned(),
            category: preset.category.to_owned(),
            patch: preset.into_patch(),
        };
        write_file(&path, &file)?;
        installed_paths.push(path);
    }
    prune_stale_community_files(&community, &installed_paths)
}

fn prune_stale_community_files(community: &Path, installed_paths: &[PathBuf]) -> io::Result<()> {
    let category_dirs = match fs::read_dir(community) {
        Ok(dirs) => dirs,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for category_dir in category_dirs.flatten() {
        let files = fs::read_dir(category_dir.path())?;
        for file in files.flatten() {
            let path = file.path();
            if path.extension().is_some_and(|extension| extension == PRESET_EXTENSION)
                && !installed_paths.contains(&path)
            {
                fs::remove_file(&path)?;
            }
        }
    }
    Ok(())
}

fn write_file(path: &Path, file: &PresetFile) -> io::Result<()> {
    let json = serde_json::to_string_pretty(file)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, json)
}

pub fn save_preset(
    root: &Path,
    category: &str,
    name: &str,
    author: &str,
    patch: &Patch,
) -> io::Result<PathBuf> {
    let display_name = name.trim();
    if display_name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "preset name is empty",
        ));
    }
    let category = if category.trim().is_empty() {
        FALLBACK_CATEGORY
    } else {
        category.trim()
    };
    let dir = root.join(USER_DIR).join(sanitize_file_name(category));
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!(
        "{}.{PRESET_EXTENSION}",
        sanitize_file_name(display_name)
    ));
    let file = PresetFile {
        format: FORMAT_VERSION,
        name: display_name.to_owned(),
        author: author.trim().to_owned(),
        category: category.to_owned(),
        patch: patch.clone(),
    };
    write_file(&path, &file)?;
    Ok(path)
}

pub fn load_preset(path: &Path) -> io::Result<PresetFile> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub fn scan() -> Vec<PresetEntry> {
    match preset_root() {
        Some(root) => scan_root(&root),
        None => Vec::new(),
    }
}

fn scan_root(root: &Path) -> Vec<PresetEntry> {
    let mut entries = Vec::new();
    for (origin, section_dir) in [
        (PresetOrigin::Community, root.join(COMMUNITY_DIR)),
        (PresetOrigin::User, root.join(USER_DIR)),
    ] {
        let Ok(section) = fs::read_dir(&section_dir) else {
            continue;
        };
        let mut categories: Vec<String> = section
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        categories.sort();
        for category in categories {
            let Ok(files) = fs::read_dir(section_dir.join(&category)) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = files
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == PRESET_EXTENSION))
                .collect();
            paths.sort();
            for path in paths {
                let file_stem = path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_default();
                let (name, author) = load_preset(&path)
                    .map(|file| (file.name, file.author))
                    .unwrap_or((file_stem.clone(), String::new()));
                entries.push(PresetEntry {
                    name,
                    author,
                    category: category.clone(),
                    origin,
                    path,
                });
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::{Grid, PlayMode};

    fn sample_patch() -> Patch {
        let mut grid = Grid::new();
        grid.set(
            0,
            Some(crate::engine::node::NodeParams::Gain { gain: 0.5 }),
            crate::engine::graph::InputSel::None,
            crate::engine::graph::InputSel::None,
        );
        Patch::new(grid)
    }

    #[test]
    fn sanitize_replaces_separators_and_trims() {
        assert_eq!(sanitize_file_name("My/Great:Preset? "), "My_Great_Preset_");
        assert_eq!(sanitize_file_name("  "), "");
        assert_eq!(sanitize_file_name("Pad - Warm"), "Pad - Warm");
    }

    #[test]
    fn save_then_scan_then_load_round_trips() {
        let root = std::env::temp_dir().join(format!("sozvezdie_bank_test_{}", fastrand::u64(..)));
        let patch = sample_patch();
        let saved_path =
            save_preset(&root, "Pads", "Test Pad", "Tester", &patch).expect("saving should work");

        let entries = scan_root(&root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Test Pad");
        assert_eq!(entries[0].author, "Tester");
        assert_eq!(entries[0].category, "Pads");
        assert_eq!(entries[0].origin, PresetOrigin::User);
        assert_eq!(entries[0].path, saved_path);

        let file = load_preset(&saved_path).expect("loading should work");
        assert_eq!(file.format, FORMAT_VERSION);
        assert_eq!(file.patch.play_mode, PlayMode::default());
        assert_eq!(file.patch.grid.cells.len(), patch.grid.cells.len());

        fs::remove_dir_all(&root).expect("cleanup should work");
    }

    #[test]
    fn empty_name_is_rejected() {
        let root = std::env::temp_dir().join(format!("sozvezdie_bank_test_{}", fastrand::u64(..)));
        assert!(save_preset(&root, "Pads", "   ", "Tester", &sample_patch()).is_err());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn stale_community_files_are_pruned() {
        let root = std::env::temp_dir().join(format!("sozvezdie_bank_test_{}", fastrand::u64(..)));
        let community = root.join(COMMUNITY_DIR);
        let category = community.join("Basses");
        fs::create_dir_all(&category).expect("category dir should be created");
        let installed = category.join("Kept_Preset.lmp");
        fs::write(&installed, "{}").expect("installed preset should be written");
        let stale = category.join("Cyber_Pulse.lmp");
        fs::write(&stale, "{}").expect("stale preset should be written");

        prune_stale_community_files(&community, std::slice::from_ref(&installed))
            .expect("prune should work");

        assert!(installed.exists());
        assert!(!stale.exists());
        fs::remove_dir_all(&root).expect("cleanup should work");
    }
}
