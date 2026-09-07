//! Port of `spider/SpiderDen.java` (per-gallery storage) and
//! `spider/SpiderInfo.java` (the `.ehviewer` metadata file).
//!
//! Layout per gallery: `{download_root}/{dirname}/####.{ext}` pages plus one
//! `.ehviewer` metadata file. Deviations from the original, kept deliberate:
//! * Page files are `{:04}.ext` per the Yomira spec (the original writes
//!   `{:08}.ext`; resume scans accept both, see [`page_index_of`]).
//! * The metadata file itself is byte-compatible with `SpiderInfo` VERSION 2
//!   and readable by the original app (version header, hex start page,
//!   gid/token/counts header, `{index} {ptoken}` lines).

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// Metadata filename inside every gallery download dir
/// (`SpiderQueen.SPIDER_INFO_FILENAME`).
pub const SPIDER_INFO_FILENAME: &str = ".ehviewer";

/// Port of `GalleryProvider2.SUPPORT_IMAGE_EXTENSIONS` (with dots).
pub const SUPPORT_IMAGE_EXTENSIONS: [&str; 5] = [".jpg", ".png", ".gif", ".webp", ".avif"];

/// Upper bound for page counts read from a metadata file, guarding against
/// corrupted files (port of `SpiderInfo.MAX_SPIDER_INFO_PAGES`).
const MAX_SPIDER_INFO_PAGES: i32 = 100_000;

/// Reject absurdly long tokens from corrupt files
/// (port of `SpiderInfo.MAX_STORED_PTOKEN_CHARS`).
const MAX_STORED_PTOKEN_CHARS: usize = 1024;

/// Port of `com.hippo.lib.yorozuya.FileUtils.sanitizeFilename`: filesystem
/// hostiles (`\ / : * ? " < > |`) and control characters become `_`.
pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            ':' | '\\' | '/' | '*' | '?' | '|' | '<' | '>' | '"' => out.push('_'),
            c if (c as u32) < 0x20 => out.push('_'),
            c => out.push(c),
        }
    }
    // Deviation: Windows refuses trailing dots/spaces in directory names.
    out.trim_end_matches(['.', ' ']).to_string()
}

/// `{gid}-{sanitized suitable title}` — port of the dirname half of
/// `SpiderDen.getGalleryDownloadDir`. The "suitable title" follows upstream
/// `EhUtils.getSuitableTitle`: the Japanese title when present, else the
/// English one (the original app's "download Japanese title" setting is not
/// ported yet; this matches its default).
pub fn suitable_dirname(gid: u64, title: &str, title_jpn: Option<&str>) -> String {
    let suitable = title_jpn
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or(title);
    sanitize_filename(&format!("{gid}-{suitable}"))
}

/// Page filename for 0-based `index` — `####.{ext}` per the Yomira spec.
pub fn page_filename(index: i32, extension: &str) -> String {
    format!("{:04}.{}", index + 1, extension.trim_start_matches('.'))
}

/// If `name` is one of our page files (`NNNN.ext`, any supported extension),
/// return its 0-based page index (the filename is 1-based, like the
/// original). Accepts both 4-digit Yomira names and 8-digit EhViewer
/// names so galleries downloaded by the original app scan correctly.
fn page_index_of(name: &str) -> Option<i32> {
    let (stem, ext) = name.rsplit_once('.')?;
    let dotted = format!(".{ext}");
    if !SUPPORT_IMAGE_EXTENSIONS.contains(&dotted.as_str()) {
        return None;
    }
    if stem.is_empty() || stem.len() > 8 || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let number = stem.parse::<i32>().ok()?;
    if number <= 0 || (stem != format!("{number:04}") && stem != format!("{number:08}")) {
        return None;
    }
    Some(number - 1)
}

/// On-disk home of one gallery's pages (port of a resolved `SpiderDen`
/// download dir).
pub struct GalleryDen {
    dir: PathBuf,
}

impl GalleryDen {
    /// Open the gallery dir under `root`, creating it if needed
    /// (`SpiderDen.prepareDownloadStorage`).
    pub fn open(root: &Path, dirname: &str) -> io::Result<Self> {
        let dir = root.join(dirname);
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Reference an existing dir without creating anything (resume scans).
    pub fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    pub fn page_path(&self, index: i32, extension: &str) -> PathBuf {
        self.dir.join(page_filename(index, extension))
    }

    /// First existing page file for `index`, trying every supported
    /// extension (port of `SpiderDen.findImageFile`).
    pub fn find_page(&self, index: i32) -> Option<PathBuf> {
        if index < 0 {
            return None;
        }
        SUPPORT_IMAGE_EXTENSIONS
            .iter()
            .flat_map(|ext| {
                [
                    self.page_path(index, ext),
                    self.dir.join(format!("{:08}{ext}", index + 1)),
                ]
            })
            .find(|path| self.is_readable_page(path))
    }

    fn is_readable_page(&self, path: &Path) -> bool {
        // A page must be a nonempty regular file inside this gallery.
        let Ok(meta) = fs::symlink_metadata(path) else {
            return false;
        };
        meta.is_file() && meta.len() > 0
    }

    pub fn available_pages(&self, page_count: i32) -> Vec<i32> {
        let mut indices = std::collections::BTreeSet::new();
        if let Ok(entries) = fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                if let Some(index) = page_index_of(name) {
                    if index < page_count && self.is_readable_page(&entry.path()) {
                        indices.insert(index);
                    }
                }
            }
        }
        indices.into_iter().collect()
    }

    /// Write a page atomically (temp file + rename) so a crash mid-write can
    /// never leave a truncated page that resume would count as finished.
    pub fn write_page(&self, index: i32, extension: &str, bytes: &[u8]) -> io::Result<PathBuf> {
        let final_path = self.page_path(index, extension);
        let tmp = self.dir.join(format!(
            ".tmp-{}-{}",
            std::process::id(),
            final_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("page")
        ));
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &final_path)?;
        Ok(final_path)
    }

    /// Number of finished pages in this dir (distinct page-file indices).
    pub fn count_pages(&self) -> i32 {
        self.available_pages(MAX_SPIDER_INFO_PAGES).len() as i32
    }

    /// Delete the whole gallery dir (`DownloadManager` file removal).
    pub fn remove_dir(&self) -> io::Result<()> {
        match fs::remove_dir_all(&self.dir) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// In-memory form of the `.ehviewer` metadata file (port of `SpiderInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiderInfo {
    /// Download progress in pages (hex-encoded on disk) — the resume point.
    pub start_page: i32,
    pub gid: i64,
    pub token: String,
    pub pages: i32,
    pub preview_pages: i32,
    pub preview_per_page: i32,
    /// Page index → ptoken, filled from detail previews and as pages are
    /// fetched. "failed" tokens are skipped, like the original write.
    pub ptoken_map: BTreeMap<i32, String>,
}

impl SpiderInfo {
    pub fn new(
        gid: i64,
        token: impl Into<String>,
        pages: i32,
        preview_pages: i32,
        preview_per_page: i32,
    ) -> Self {
        Self {
            start_page: 0,
            gid,
            token: token.into(),
            pages,
            preview_pages,
            preview_per_page,
            ptoken_map: BTreeMap::new(),
        }
    }

    /// Serialize in `SpiderInfo` VERSION 2 layout (byte-compatible with the
    /// original `SpiderInfo.write`).
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        writeln!(w, "VERSION2")?;
        writeln!(w, "{:08x}", self.start_page.max(0))?;
        writeln!(w, "{}", self.gid)?;
        writeln!(w, "{}", self.token)?;
        writeln!(w, "1")?;
        writeln!(w, "{}", self.preview_pages)?;
        writeln!(w, "{}", self.preview_per_page)?;
        writeln!(w, "{}", self.pages)?;
        for (index, ptoken) in &self.ptoken_map {
            if ptoken == "failed" || ptoken.is_empty() {
                continue;
            }
            writeln!(w, "{} {}", index, ptoken)?;
        }
        w.flush()
    }

    /// Write the `.ehviewer` file inside `dir`, returning its path.
    pub fn write_to_dir(&self, dir: &Path) -> io::Result<PathBuf> {
        let path = dir.join(SPIDER_INFO_FILENAME);
        let temporary = dir.join(format!(".ehviewer.tmp-{}", std::process::id()));
        // The reader may inspect this header while a download advances.
        // Publish a complete replacement so it never observes a truncated file.
        let result = (|| {
            let mut file = fs::File::create(&temporary)?;
            self.write(&mut file)?;
            drop(file);
            fs::rename(&temporary, &path)
        })();
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        Ok(path)
    }

    /// Read the full file including the ptoken map.
    pub fn read(path: &Path) -> io::Result<Option<SpiderInfo>> {
        Self::read_impl(path, true)
    }

    /// Parse the header only (gid/token/pages counts); the ptoken map is left
    /// empty. Port of `SpiderInfo.readHeader`.
    pub fn read_header(path: &Path) -> io::Result<Option<SpiderInfo>> {
        Self::read_impl(path, false)
    }

    fn read_impl(path: &Path, include_ptokens: bool) -> io::Result<Option<SpiderInfo>> {
        let text = if include_ptokens {
            let Ok(text) = fs::read_to_string(path) else {
                return Ok(None);
            };
            text
        } else {
            let Ok(file) = fs::File::open(path) else {
                return Ok(None);
            };
            // VERSION 2 has eight header lines; VERSION 1 has seven.
            // The size bound also keeps malformed header lines from growing
            // allocations without limit. Tokens are normally ten characters.
            let mut reader = BufReader::new(file).take(8 * 1024);
            let mut header = String::new();
            for _ in 0..8 {
                if reader.read_line(&mut header)? == 0 {
                    break;
                }
            }
            header
        };
        let mut lines = text.lines();

        // Version line: "VERSION<n>" or a v1 file whose first line is the
        // hex start page (port of SpiderInfo.getVersion).
        let Some(first) = lines.next() else {
            return Ok(None);
        };
        let (version, start_line) = if let Some(rest) = first.strip_prefix("VERSION") {
            let version: i32 = rest.trim().parse().unwrap_or(-1);
            if version != 2 {
                return Ok(None);
            }
            (version, lines.next().unwrap_or(""))
        } else {
            (1, first)
        };

        let Some(line) = lines.next() else {
            return Ok(None);
        };
        let Some(gid) = line.trim().parse::<i64>().ok() else {
            return Ok(None);
        };
        let Some(token) = lines.next() else {
            return Ok(None);
        };
        let token = token.to_string();
        lines.next(); // ignored legacy line (the "1" the writer emits)

        let Some(line) = lines.next() else {
            return Ok(None);
        };
        let Some(preview_pages) = line.trim().parse::<i32>().ok() else {
            return Ok(None);
        };
        let preview_per_page = if version == 1 {
            0
        } else {
            let Some(line) = lines.next() else {
                return Ok(None);
            };
            let Some(v) = line.trim().parse::<i32>().ok() else {
                return Ok(None);
            };
            v
        };
        let Some(line) = lines.next() else {
            return Ok(None);
        };
        let Some(pages) = line.trim().parse::<i32>().ok() else {
            return Ok(None);
        };
        if pages <= 0 || pages > MAX_SPIDER_INFO_PAGES {
            return Ok(None);
        }

        let mut ptoken_map = BTreeMap::new();
        if include_ptokens {
            for _ in 0..pages {
                let Some(line) = lines.next() else { break };
                let Some(pos) = line.find(' ') else { continue };
                let Ok(index) = line[..pos].trim().parse::<i32>() else {
                    continue;
                };
                let ptoken = line[pos + 1..].trim();
                if !ptoken.is_empty() && ptoken.chars().count() <= MAX_STORED_PTOKEN_CHARS {
                    ptoken_map.insert(index, ptoken.to_string());
                }
            }
        }

        Ok(Some(SpiderInfo {
            start_page: i64::from_str_radix(start_line.trim(), 16)
                .unwrap_or(0)
                .max(0) as i32,
            gid,
            token,
            pages,
            preview_pages,
            preview_per_page,
            ptoken_map,
        }))
    }
}

#[cfg(test)]
#[path = "den_tests.rs"]
mod tests;
