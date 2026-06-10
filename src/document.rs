//! Document state — raw Markdown content, on-disk path, and file I/O.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The in-memory representation of an open binder document.
#[derive(Debug, Clone)]
pub struct Document {
    /// The current Markdown source text.
    pub content: String,
    /// The on-disk path, if the document has ever been saved.
    pub path: Option<PathBuf>,
    /// True when the in-memory content differs from what is on disk.
    pub is_modified: bool,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// Create a new, empty, unsaved document.
    pub fn new() -> Self {
        Self {
            content: String::new(),
            path: None,
            is_modified: false,
        }
    }

    /// Load a document from a file on disk.
    pub fn from_file(path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self {
            content,
            path: Some(path.to_path_buf()),
            is_modified: false,
        })
    }

    /// Save to the current path. Returns an error if no path is set.
    pub fn save(&mut self) -> io::Result<()> {
        match self.path.clone() {
            Some(p) => self.save_to(p),
            None => Err(io::Error::other("no path set — use save_to")),
        }
    }

    /// Save to a specific path and update the document's stored path.
    pub fn save_to(&mut self, path: PathBuf) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &self.content)?;
        self.path = Some(path);
        self.is_modified = false;
        Ok(())
    }

    /// The bare filename, or `"Untitled"` for unsaved documents.
    pub fn title(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// The full window title, including a `•` prefix when modified.
    pub fn window_title(&self) -> String {
        let name = self.title();
        if self.is_modified {
            format!("• {name} — Swatchbook")
        } else {
            format!("{name} — Swatchbook")
        }
    }

    // ── Auto-save / crash recovery ────────────────────────────────────────────

    /// `$XDG_DATA_HOME/swatchbook/autosave.md`
    pub fn autosave_path() -> PathBuf {
        data_dir().join("autosave.md")
    }

    /// Sentinel file written on startup; removed on clean exit.
    pub fn sentinel_path() -> PathBuf {
        data_dir().join(".running")
    }

    /// Write the current content to the auto-save path.
    pub fn write_autosave(&self) -> io::Result<()> {
        let path = Self::autosave_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &self.content)
    }

    /// Write the crash-detection sentinel file.
    pub fn write_sentinel() -> io::Result<()> {
        let path = Self::sentinel_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, b"running")
    }

    /// Remove the sentinel file on clean exit.
    pub fn clear_sentinel() -> io::Result<()> {
        let path = Self::sentinel_path();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// True if a previous run crashed (sentinel exists and autosave exists).
    pub fn has_crash_recovery() -> bool {
        Self::sentinel_path().exists() && Self::autosave_path().exists()
    }

    /// Load the autosave as an unnamed document for crash recovery.
    pub fn recover() -> io::Result<Self> {
        let content = fs::read_to_string(Self::autosave_path())?;
        Ok(Self {
            content,
            path: None,
            is_modified: true,
        })
    }
}

fn data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs_or_home().join(".local").join("share"));
    base.join("swatchbook")
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
