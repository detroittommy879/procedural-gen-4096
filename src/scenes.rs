use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use crate::gpu::Settings;

pub struct Scene {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub defaults: Settings,
    pub badge: [u8; 3],
}

const BUNDLED: [(&str, &str, [u8; 3], Settings); 4] = [
    (
        "alpine-lake",
        include_str!("../scenes/alpine-lake.wgsl"),
        [129, 209, 221],
        Settings {
            seed: 27,
            light: 0.54,
            color: 0.52,
            camera: 0.43,
            detail: 0.57,
        },
    ),
    (
        "red-arch",
        include_str!("../scenes/red-arch.wgsl"),
        [248, 162, 115],
        Settings {
            seed: 438,
            light: 0.28,
            color: 0.58,
            camera: 0.81,
            detail: 0.63,
        },
    ),
    (
        "lavender-fields",
        include_str!("../scenes/lavender-fields.wgsl"),
        [202, 159, 249],
        Settings {
            seed: 107,
            light: 0.61,
            color: 0.67,
            camera: 0.49,
            detail: 0.64,
        },
    ),
    (
        "neon-passage",
        include_str!("../scenes/neon-passage.wgsl"),
        [123, 145, 250],
        Settings {
            seed: 915,
            light: 0.44,
            color: 0.58,
            camera: 0.80,
            detail: 0.53,
        },
    ),
];

const CUSTOM_DEFAULTS: Settings = Settings {
    seed: 1,
    light: 0.5,
    color: 0.5,
    camera: 0.5,
    detail: 0.5,
};

pub struct Library;

impl Library {
    pub fn bundled() -> Vec<Scene> {
        BUNDLED
            .into_iter()
            .map(|(id, source, badge, defaults)| {
                make_scene(id.to_owned(), source.to_owned(), badge, defaults)
            })
            .collect()
    }
    pub fn load(path: &Path) -> io::Result<Vec<Scene>> {
        let mut files = BTreeMap::new();
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    let file = entry?.path();
                    if file.extension().is_some_and(|ext| ext == "wgsl")
                        && let Some(id) = file.file_stem().and_then(|stem| stem.to_str())
                    {
                        files.insert(
                            id.to_owned(),
                            fs::read_to_string(&file).map_err(|error| {
                                io::Error::new(error.kind(), format!("{}: {error}", file.display()))
                            })?,
                        );
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let mut scenes = Vec::new();
        for (id, bundled, badge, defaults) in BUNDLED {
            let source = files.remove(id).unwrap_or_else(|| bundled.to_owned());
            scenes.push(make_scene(id.to_owned(), source, badge, defaults));
        }
        scenes.extend(
            files
                .into_iter()
                .map(|(id, source)| make_scene(id, source, [166, 216, 178], CUSTOM_DEFAULTS)),
        );
        Ok(scenes)
    }
}

fn make_scene(id: String, source: String, badge: [u8; 3], defaults: Settings) -> Scene {
    let metadata = |key| {
        source
            .lines()
            .take(12)
            .find_map(|line| line.trim().strip_prefix(key))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let name = metadata("// name:").unwrap_or_else(|| id.replace(['-', '_'], " "));
    let description = metadata("// description:").unwrap_or_default();
    Scene {
        id,
        name,
        description,
        source,
        defaults,
        badge,
    }
}

pub fn directory() -> PathBuf {
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let local = current.join("scenes");
    if local.is_dir() {
        return local;
    }
    let Ok(exe) = std::env::current_exe() else {
        return local;
    };
    let Some(bin_dir) = exe.parent() else {
        return local;
    };
    let adjacent = bin_dir.join("scenes");
    if adjacent.is_dir() {
        return adjacent;
    }
    if let Some(project) = bin_dir.parent().and_then(Path::parent) {
        let source = project.join("scenes");
        if project.join("Cargo.toml").is_file() && source.is_dir() {
            return source;
        }
    }
    adjacent
}
