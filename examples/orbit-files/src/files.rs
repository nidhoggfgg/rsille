//! Filesystem operations, deliberately independent of terminal rendering.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static SERIAL: AtomicU64 = AtomicU64::new(0);
pub fn token() -> String {
    format!(
        "{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    )
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub dir: bool,
    pub link: bool,
    pub size: u64,
    pub modified: u64,
}
impl Entry {
    pub fn kind(&self) -> &str {
        if self.link {
            "LINK"
        } else if self.dir {
            "FOLDER"
        } else {
            match self.path.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "rs" | "toml" | "json" | "css" | "js" | "py" => "CODE",
                "png" | "jpg" | "svg" | "webp" => "IMAGE",
                "md" | "txt" => "TEXT",
                "mp3" | "wav" | "flac" => "AUDIO",
                "zip" | "gz" | "tar" => "ARCHIVE",
                _ => "FILE",
            }
        }
    }
}
pub fn read_dir(path: &Path) -> io::Result<Vec<Entry>> {
    fs::read_dir(path)?
        .map(|item| {
            let item = item?;
            let path = item.path();
            let meta = fs::symlink_metadata(&path)?;
            Ok(Entry {
                name: item.file_name().to_string_lossy().into_owned(),
                dir: meta.is_dir(),
                link: meta.is_symlink(),
                size: meta.len(),
                path,
                modified: meta
                    .modified()
                    .unwrap_or(UNIX_EPOCH)
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            })
        })
        .filter(|item| !matches!(item, Ok(e) if e.name.starts_with(".orbit-trash-")))
        .collect()
}
pub fn valid_name(name: &str) -> io::Result<&str> {
    let name = name.trim();
    let mut parts = Path::new(name).components();
    if name.is_empty()
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
        || name.starts_with(".orbit-trash-")
        || !matches!(parts.next(), Some(Component::Normal(_)))
        || parts.next().is_some()
    {
        return Err(io::Error::other(
            "Use a single file name, without /, \\ or ..",
        ));
    }
    Ok(name)
}
pub fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// Linux provides an atomic no-replace rename, including for directories.
pub fn move_new(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        rustix::fs::renameat_with(
            rustix::fs::CWD,
            from,
            rustix::fs::CWD,
            to,
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(Into::into)
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Refuse rather than silently replace another file on unsupported systems.
        let _ = (from, to);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "This example requires Linux for atomic no-replace moves",
        ))
    }
}
pub fn copy_new(from: &Path, to: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(from)?;
    if meta.is_symlink() {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(fs::read_link(from)?, to)
        }
        #[cfg(not(unix))]
        {
            Err(io::Error::other(
                "Copying symbolic links is unsupported on this platform",
            ))
        }
    } else if meta.is_dir() {
        fs::create_dir(to)?;
        let result = (|| {
            for item in fs::read_dir(from)? {
                let item = item?;
                copy_new(&item.path(), &to.join(item.file_name()))?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(to);
        }
        result
    } else if meta.is_file() {
        let mut source = File::open(from)?;
        let mut target = OpenOptions::new().write(true).create_new(true).open(to)?;
        let result = io::copy(&mut source, &mut target).and_then(|_| target.sync_all());
        if result.is_err() {
            let _ = fs::remove_file(to);
        }
        result
    } else {
        Err(io::Error::other(
            "Special files (devices, sockets, pipes) cannot be copied",
        ))
    }
}
fn copy_target(dir: &Path, source: &Path) -> PathBuf {
    let name = source.file_name().unwrap_or_default();
    let target = dir.join(name);
    if !exists(&target) {
        return target;
    }
    let stem = source.file_stem().unwrap_or(name).to_string_lossy();
    let ext = source
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for n in 1.. {
        let candidate = dir.join(format!("{stem} (copy {n}){ext}"));
        if !exists(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Copy,
    Move,
    Trash,
}
#[derive(Clone, Debug)]
pub struct Job {
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub operation: Operation,
}
#[derive(Default, Debug)]
pub struct Outcome {
    pub completed: usize,
    /// (current path, original path), for non-destructive undo.
    pub undo: Vec<(PathBuf, PathBuf)>,
    pub errors: Vec<String>,
}
impl Job {
    pub fn run(&self, mut progress: impl FnMut(usize, usize)) -> Outcome {
        let mut outcome = Outcome::default();
        for (index, source) in self.sources.iter().enumerate() {
            let result = (|| {
                let destination = if self.operation == Operation::Trash {
                    let trash = source
                        .parent()
                        .ok_or_else(|| io::Error::other("Cannot trash a filesystem root"))?
                        .join(format!(".orbit-trash-{}", token()));
                    fs::create_dir(&trash)?;
                    trash.join(source.file_name().unwrap_or_default())
                } else {
                    let dir = fs::canonicalize(&self.destination)?;
                    let meta = fs::symlink_metadata(source)?;
                    if meta.is_dir() && dir.starts_with(fs::canonicalize(source)?) {
                        return Err(io::Error::other("Cannot put a directory inside itself"));
                    }
                    if self.operation == Operation::Copy {
                        copy_target(&dir, source)
                    } else {
                        dir.join(source.file_name().unwrap_or_default())
                    }
                };
                let result = if self.operation == Operation::Copy {
                    copy_new(source, &destination)
                } else {
                    move_new(source, &destination)
                };
                if result.is_err() && self.operation == Operation::Trash {
                    if let Some(parent) = destination.parent() {
                        let _ = fs::remove_dir(parent);
                    }
                }
                result?;
                if self.operation != Operation::Copy {
                    outcome.undo.push((destination, source.clone()));
                }
                outcome.completed += 1;
                Ok::<_, io::Error>(())
            })();
            if let Err(error) = result {
                outcome
                    .errors
                    .push(format!("{}: {error}", source.display()));
            }
            progress(index + 1, self.sources.len());
        }
        outcome
    }
}
pub fn preview(path: &Path) -> String {
    let result = (|| {
        let meta = fs::symlink_metadata(path)?;
        if meta.is_symlink() {
            return Ok(format!(
                "Symbolic link\n\n{}",
                fs::read_link(path)?.display()
            ));
        }
        if meta.is_dir() {
            let entries = read_dir(path)?;
            return Ok(format!(
                "{} items\n\n{}",
                entries.len(),
                entries
                    .iter()
                    .take(18)
                    .map(|e| format!("{} {}", if e.dir { "+" } else { "·" }, e.name))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        if !meta.is_file() {
            return Ok("Special file · no preview".into());
        }
        let mut bytes = Vec::new();
        File::open(path)?.take(4096).read_to_end(&mut bytes)?;
        if bytes.contains(&0) {
            return Ok("Binary file\n\nNo text preview available.".into());
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    })();
    result.unwrap_or_else(|e: io::Error| format!("Preview unavailable: {e}"))
}

/// Own only the temporary directory created by this process.
pub struct Demo(pub PathBuf);
impl Demo {
    pub fn new() -> io::Result<Self> {
        let root = std::env::temp_dir().join(format!("rsille-orbit-{}", token()));
        fs::create_dir(&root)?;
        let demo = Self(root);
        for folder in [
            "Projects",
            "Design",
            "Documents",
            "Music",
            "Photos",
            "Archive",
        ] {
            fs::create_dir(demo.0.join(folder))?;
        }
        for (name, content) in [
            ("Welcome.md", "# Welcome to Orbit\n\nA little space for your files.\nBuilt with Rust + rsille.\n\nDrag empty space to select.\nCtrl+click to collect files.\nDrag a selection onto a folder.\nHold Ctrl to copy instead.\n\nCtrl+C / X / V · copy / cut / paste\nCtrl+Z · undo moves, renames, trash\nF2 · rename   F5 · refresh\nF1 · all shortcuts\n"),
            ("Roadmap.md", "# Next horizons\n\n[ ] Sketch the impossible\n[x] Make room for ideas\n[ ] Ship something delightful\n\n## October\nA calmer home for busy files."),
            ("Notes.txt", "星光、文件与一点点秩序。\n\n中文文件名与 Unicode 预览均可使用。"),
            ("theme.json", "{\n  \"name\": \"Midnight orbit\",\n  \"accent\": \"#83e6ce\",\n  \"background\": \"#101622\"\n}"),
            ("main.rs", "fn main() {\n    println!(\"Make useful things feel delightful.\");\n}"),
            ("Design/palette.css", ":root {\n  --mint: #83e6ce;\n  --lavender: #b5a4f5;\n  --gold: #f4ce8a;\n}"),
            ("Projects/Readme.md", "# Project shelf\n\nDrop files here to organize them."),
            ("Documents/旅行计划.md", "# 旅行计划\n\n山川与湖泊，下一站见。"),
            (".hidden-note", "You found the hidden file."),
        ] { fs::write(demo.0.join(name), content)?; }
        for i in 1..=14 {
            fs::write(
                demo.0.join(format!("Study-{i:02}.md")),
                format!("# Study {i:02}\n\nSmall ideas, wide horizons.\n"),
            )?;
        }
        Ok(demo)
    }
}
impl Drop for Demo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
