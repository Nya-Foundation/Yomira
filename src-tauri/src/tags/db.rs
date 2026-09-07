// Portions adapted from EhViewer / Ehviewer_CN_SXJ.
// Copyright 2014-2019 Hippo Seven and upstream contributors.
// Original portions: Apache-2.0; see licenses/Apache-2.0.txt and NOTICE.
// Modified for Yomira: Rust/Tauri port, async I/O and application integration.

//! Packed tag-translation database: format parsing and suggestion matching.
//!
//! This is a Rust port of the original Android implementation in
//! `com.hippo.ehviewer.client.EhTagDatabase` (Ehviewer_CN_SXJ fork) and its
//! `Tag` data bean. The packed artifact is the one the original app downloads
//! (see `download.rs`): a 4-byte big-endian length header followed by that
//! many bytes of UTF-8 text. The text is a `\n`-separated list of lines, each
//! line being `<english>\r<base64(utf8 translation)>`. The english side uses
//! short namespace prefixes (`f:`, `a:`, ...) that are expanded to full
//! namespace names through the same prefix table as the original.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;

/// namespace → short prefix (port of `NAMESPACE_TO_PREFIX`).
pub const NAMESPACE_TO_PREFIX: &[(&str, &str)] = &[
    ("rows", "n:"),
    ("artist", "a:"),
    ("cosplayer", "cos:"),
    ("character", "c:"),
    ("female", "f:"),
    ("group", "g:"),
    ("language", "l:"),
    ("location", "loc:"),
    ("male", "m:"),
    ("misc", ""),
    ("mixed", "x:"),
    ("other", "o:"),
    ("parody", "p:"),
    ("reclass", "r:"),
];

/// Original suggestion cap (`total >= 40` in `searchTag`).
pub const MAX_SUGGESTIONS: usize = 40;

/// A single parsed tag entry. `namespace` is always the full namespace name
/// (prefixes are expanded at parse time, as the original does).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagEntry {
    pub namespace: String,
    pub name: String,
    /// UTF-8 translation. The original stores the literal string "null"
    /// for lines without a translation; we keep that behaviour.
    pub translated: String,
}

/// One suggestion. `name` empty means "namespace suggestion" (the query is a
/// bare prefix of a namespace and picking it inserts `namespace:`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub namespace: String,
    pub name: String,
    pub translated: String,
}

/// In-memory tag database.
pub struct TagDatabase {
    /// Raw packed payload, kept for the binary-search translation lookup
    /// (exact port of the original `search(byte[], byte[])`).
    raw: Vec<u8>,
    entries: Vec<TagEntry>,
}

impl TagDatabase {
    /// Parse a packed artifact (4-byte BE length header + UTF-8 payload),
    /// exactly like the original constructor.
    pub fn from_packed(packed: &[u8]) -> Result<TagDatabase, String> {
        if packed.len() < 4 {
            return Err("packed tag db: missing length header".to_string());
        }
        let total = u32::from_be_bytes([packed[0], packed[1], packed[2], packed[3]]) as usize;
        if packed.len() - 4 != total {
            return Err(format!(
                "packed tag db: length header says {total} bytes, have {}",
                packed.len() - 4
            ));
        }
        let text = std::str::from_utf8(&packed[4..]).map_err(|e| format!("packed tag db: {e}"))?;
        Ok(TagDatabase::from_text(text))
    }

    /// Parse the unpacked text form (lines split on `\n`). Splitting this way
    /// keeps behaviour identical to the original `String.split("\n")`,
    /// including a trailing newline producing one empty (untranslated) entry.
    fn from_text(text: &str) -> TagDatabase {
        let mut entries = Vec::new();
        for line in text.split('\n') {
            entries.push(parse_tag_line(line));
        }
        TagDatabase {
            raw: text.as_bytes().to_vec(),
            entries,
        }
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Exact binary-search translation lookup, ported from the original
    /// `search(byte[] tags, byte[] tag)`: the payload is sorted, entries are
    /// `\n`-delimited and the key compared against the bytes before the `\r`.
    pub fn translate(&self, tag: &str) -> Option<String> {
        let tags = &self.raw;
        let needle = tag.as_bytes();
        if needle.is_empty() {
            return None;
        }
        let mut low: i64 = 0;
        let mut high: i64 = tags.len() as i64;
        while low < high {
            // Look for the starting '\n' (the original walks `start` down to
            // just after a '\n', or to 0 when the entry is the first line).
            let mut s = (low + high) / 2;
            while s > -1 && tags[s as usize] != b'\n' {
                s -= 1;
            }
            s += 1;
            let start = s as usize;

            // Look for the middle '\r'.
            let mut middle = 1usize;
            while start + middle < tags.len() && tags[start + middle] != b'\r' {
                middle += 1;
            }
            if start + middle >= tags.len() {
                return None;
            }

            // Look for the ending '\n'.
            let mut end = middle + 1;
            while start + end < tags.len() && tags[start + end] != b'\n' {
                end += 1;
            }
            if start + end >= tags.len() {
                return None;
            }

            // Compare needle with tags[start..start+middle].
            let mut compare: i32;
            let mut tag_index = 0usize;
            let mut cur_index = start;
            loop {
                let tag_byte = needle[tag_index] as i32;
                let cur_byte = tags[cur_index] as i32;
                compare = tag_byte - cur_byte;
                if compare != 0 {
                    break;
                }
                tag_index += 1;
                cur_index += 1;
                if tag_index == needle.len() && cur_index == start + middle {
                    break;
                }
                if tag_index == needle.len() {
                    compare = -1;
                    break;
                }
                if cur_index == start + middle {
                    compare = 1;
                    break;
                }
            }

            if compare < 0 {
                high = start as i64 - 1;
            } else if compare > 0 {
                low = (start + end + 1) as i64;
            } else {
                let b64 = &tags[start + middle + 1..start + end];
                return String::from_utf8(BASE64.decode(b64).ok()?).ok();
            }
        }
        None
    }

    /// Prefix suggestions, port of `suggest`/`searchTag` with the original
    /// cap of 40. Semantics:
    /// * `namespace:partial` narrows to that namespace (short prefixes like
    ///   `f:` are expanded first, as when parsing) and matches the tag name
    ///   or the translation by case-insensitive-ASCII prefix.
    /// * a bare prefix matches tag names/translations by prefix and, in
    ///   addition, yields namespace suggestions for matching namespaces.
    pub fn suggest(&self, keyword: &str) -> Vec<Suggestion> {
        let keyword = keyword.trim();
        if keyword.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<Suggestion> = Vec::new();

        if let Some(colon) = keyword.find(':') {
            let ns_query = &keyword[..colon];
            let partial = &keyword[colon + 1..];
            let narrowed: Vec<&str> = match expand_namespace(ns_query) {
                Some(ns) => vec![ns],
                None => NAMESPACE_TO_PREFIX
                    .iter()
                    .map(|(ns, _)| *ns)
                    .filter(|ns| starts_with_ascii_case(ns, ns_query))
                    .collect(),
            };
            for entry in &self.entries {
                if out.len() >= MAX_SUGGESTIONS {
                    break;
                }
                if !narrowed.iter().any(|ns| *ns == entry.namespace) {
                    continue;
                }
                if partial.is_empty()
                    || starts_with_ascii_case(&entry.name, partial)
                    || starts_with_ascii_case(&entry.translated, partial)
                {
                    out.push(Suggestion {
                        namespace: entry.namespace.clone(),
                        name: entry.name.clone(),
                        translated: entry.translated.clone(),
                    });
                }
            }
        } else {
            for (ns, _) in NAMESPACE_TO_PREFIX {
                if out.len() >= MAX_SUGGESTIONS {
                    break;
                }
                if starts_with_ascii_case(ns, keyword) {
                    out.push(Suggestion {
                        namespace: ns.to_string(),
                        name: String::new(),
                        translated: String::new(),
                    });
                }
            }
            for entry in &self.entries {
                if out.len() >= MAX_SUGGESTIONS {
                    break;
                }
                if starts_with_ascii_case(&entry.name, keyword)
                    || starts_with_ascii_case(&entry.translated, keyword)
                {
                    out.push(Suggestion {
                        namespace: entry.namespace.clone(),
                        name: entry.name.clone(),
                        translated: entry.translated.clone(),
                    });
                }
            }
        }
        out
    }
}

/// Port of the original `parseTag`: `english\rbase64(translation)` lines;
/// short prefixes in the english side are expanded to full namespaces.
fn parse_tag_line(line: &str) -> TagEntry {
    let mut parts = line.split('\r');
    let english = parts.next().unwrap_or("");
    if let Some(b64) = parts.next() {
        let translated = String::from_utf8(BASE64.decode(b64).unwrap_or_default())
            .unwrap_or_else(|_| "null".to_string());
        let mut e = english.splitn(2, ':');
        let head = e.next().unwrap_or("");
        if let Some(rest) = e.next() {
            let key = format!("{head}:");
            let ns = prefix_to_namespace(&key).unwrap_or(english);
            return TagEntry {
                namespace: ns.to_string(),
                name: rest.to_string(),
                translated,
            };
        }
        // No colon: misc-prefixed bare tag.
        return TagEntry {
            namespace: "misc".to_string(),
            name: english.to_string(),
            translated,
        };
    }
    // Line without '\r': untranslated, as in the original (`Tag(source,
    // "null")`).
    TagEntry {
        namespace: "misc".to_string(),
        name: english.to_string(),
        translated: "null".to_string(),
    }
}

/// Port of `prefixToNamespace`.
pub fn prefix_to_namespace(prefix: &str) -> Option<&'static str> {
    NAMESPACE_TO_PREFIX
        .iter()
        .find(|(_, p)| *p == prefix)
        .map(|(ns, _)| *ns)
}

/// Reverse of [`prefix_to_namespace`]: full namespace → short prefix
/// (including the colon), for building packed-DB lookup keys. `"misc"` maps
/// to the empty prefix, so its tags are stored as bare names.
pub fn namespace_to_prefix(namespace: &str) -> Option<&'static str> {
    NAMESPACE_TO_PREFIX
        .iter()
        .find(|(ns, _)| *ns == namespace)
        .map(|(_, p)| *p)
}

/// Resolve a namespace query (either a full namespace like `female` or a
/// short prefix like `f`/`f:`) to a single full namespace.
fn expand_namespace(query: &str) -> Option<&'static str> {
    let q = query.strip_suffix(':').unwrap_or(query);
    if let Some((ns, _)) = NAMESPACE_TO_PREFIX.iter().find(|(ns, _)| *ns == q) {
        return Some(ns);
    }
    // Single short prefix letter(s), e.g. "f" or "loc".
    let with_colon = format!("{q}:");
    prefix_to_namespace(&with_colon)
}

fn starts_with_ascii_case(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    !n.is_empty() && h.len() >= n.len() && h[..n.len()].eq_ignore_ascii_case(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic packed blob replicating the real format:
    /// BE u32 payload length + UTF-8 lines of `english\rb64(translation)`.
    fn pack(lines: &[(&str, &str)]) -> Vec<u8> {
        let mut text = String::new();
        for (en, zh) in lines {
            text.push_str(en);
            text.push('\r');
            text.push_str(&BASE64.encode(zh.as_bytes()));
            text.push('\n');
        }
        let mut out = (text.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(text.as_bytes());
        out
    }

    fn db(lines: &[(&str, &str)]) -> TagDatabase {
        TagDatabase::from_packed(&pack(lines)).expect("synthetic blob parses")
    }

    #[test]
    fn unpacks_prefix_namespaces() {
        let db = db(&[
            ("f:sole_female", "唯一女性"),
            ("a:shindol", "新堂エル"),
            ("loc:beach", "沙滩"),
            ("full_color", "全彩"),
        ]);
        // The real artifact ends with a trailing '\n'; like the original's
        // String.split("\n"), that yields one final empty untranslated entry.
        assert_eq!(db.entry_count(), 5);
        assert_eq!(
            db.entries[0],
            TagEntry {
                namespace: "female".into(),
                name: "sole_female".into(),
                translated: "唯一女性".into(),
            }
        );
        assert_eq!(db.entries[1].namespace, "artist");
        assert_eq!(db.entries[2].namespace, "location");
        // Bare tags land in misc.
        assert_eq!(db.entries[3].namespace, "misc");
        assert_eq!(db.entries[3].name, "full_color");
        assert_eq!(db.entries[4].name, "");
        assert_eq!(db.entries[4].translated, "null");
    }

    #[test]
    fn translate_binary_search_matches_original_test() {
        // Same data as the upstream EhTagDatabaseTest.
        let db = db(&[
            ("1", "a"),
            ("12", "ab"),
            ("123", "abc"),
            ("1234", "abcd"),
            ("a", "1"),
            ("ab", "12"),
            ("abc", "123"),
            ("abcd", "1234"),
        ]);
        assert_eq!(db.translate("1").as_deref(), Some("a"));
        assert_eq!(db.translate("12").as_deref(), Some("ab"));
        assert_eq!(db.translate("123").as_deref(), Some("abc"));
        assert_eq!(db.translate("1234").as_deref(), Some("abcd"));
        assert_eq!(db.translate("a").as_deref(), Some("1"));
        assert_eq!(db.translate("ab").as_deref(), Some("12"));
        assert_eq!(db.translate("abc").as_deref(), Some("123"));
        assert_eq!(db.translate("abcd").as_deref(), Some("1234"));
        assert_eq!(db.translate("21"), None);
    }

    #[test]
    fn loc_prefix_looks_up_and_suggests_full_namespace() {
        // Mirrors the upstream test `locPrefixLooksUpAndSuggestsFullNamespace`.
        let db = db(&[("loc:beach", "沙滩")]);
        assert_eq!(db.translate("loc:beach").as_deref(), Some("沙滩"));
        let hints = db.suggest("location:beach");
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].namespace, "location");
        assert_eq!(hints[0].name, "beach");
        assert_eq!(hints[0].translated, "沙滩");
    }

    #[test]
    fn suggest_is_case_insensitive_ascii_prefix() {
        let db = db(&[
            ("f:big_breasts", "巨乳"),
            ("f:blowjob", "口交"),
            ("m:bigs", "大数据"),
        ]);
        let hints = db.suggest("f:BIG");
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].name, "big_breasts");
        assert!(hints.iter().all(|h| h.namespace == "female"));
        let bare = db.suggest("Blow");
        assert_eq!(bare.len(), 1);
        assert_eq!(bare[0].name, "blowjob");
    }

    #[test]
    fn suggest_matches_translated_prefix_for_chinese_input() {
        let db = db(&[("f:sole_female", "唯一女性"), ("m:sole_male", "唯一男性")]);
        let hints = db.suggest("唯一");
        assert_eq!(hints.len(), 2);
        assert!(hints.iter().any(|h| h.namespace == "female"));
        assert!(hints.iter().any(|h| h.namespace == "male"));
    }

    #[test]
    fn namespace_narrowing_with_short_and_full_prefixes() {
        let db = db(&[
            ("f:yuri", "百合"),
            ("m:yaoi", "耽美"),
            ("f:yandere", "病娇"),
        ]);
        let full = db.suggest("female:y");
        assert_eq!(full.len(), 2);
        assert!(full.iter().all(|h| h.namespace == "female"));
        // Short prefix expands, partial may be empty (whole namespace).
        let short = db.suggest("m:");
        assert_eq!(short.len(), 1);
        assert_eq!(short[0].name, "yaoi");
        // Partial namespace narrows by prefix.
        let partial_ns = db.suggest("ma:y");
        assert_eq!(partial_ns.len(), 1);
        assert_eq!(partial_ns[0].namespace, "male");
    }

    #[test]
    fn bare_prefix_yields_namespace_suggestions() {
        let db = db(&[("f:yuri", "百合"), ("m:yaoi", "耽美")]);
        let hints = db.suggest("fem");
        assert!(hints
            .iter()
            .any(|h| h.namespace == "female" && h.name.is_empty()));
        // Namespace suggestions and tag hits share the original cap.
        assert!(hints.len() <= MAX_SUGGESTIONS);
    }

    #[test]
    fn suggest_caps_at_original_limit() {
        let lines: Vec<(String, String)> = (0..60)
            .map(|i| (format!("f:tag_{i:02}"), format!("标签{i:02}")))
            .collect();
        let lines: Vec<(&str, &str)> = lines
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let db = db(&lines);
        assert_eq!(db.suggest("tag_").len(), MAX_SUGGESTIONS);
        assert_eq!(MAX_SUGGESTIONS, 40);
    }

    #[test]
    fn suggest_ignores_empty_keyword() {
        let db = db(&[("f:yuri", "百合")]);
        assert!(db.suggest("").is_empty());
        assert!(db.suggest("   ").is_empty());
    }

    #[test]
    fn rejects_truncated_or_corrupt_blobs() {
        assert!(TagDatabase::from_packed(&[0, 0, 0, 10, b'a']).is_err());
        let mut blob = pack(&[("f:yuri", "百合")]);
        blob.truncate(2);
        assert!(TagDatabase::from_packed(&blob).is_err());
    }

    #[test]
    fn namespace_prefix_round_trips() {
        assert_eq!(namespace_to_prefix("female"), Some("f:"));
        assert_eq!(namespace_to_prefix("cosplayer"), Some("cos:"));
        assert_eq!(namespace_to_prefix("location"), Some("loc:"));
        // misc carries the empty prefix: bare-name keys.
        assert_eq!(namespace_to_prefix("misc"), Some(""));
        assert_eq!(namespace_to_prefix("bogus"), None);
        // Reverse direction agrees for every table entry (the empty misc
        // prefix round-trips to "misc").
        for (ns, prefix) in NAMESPACE_TO_PREFIX {
            assert_eq!(namespace_to_prefix(ns), Some(*prefix));
            assert_eq!(prefix_to_namespace(prefix), Some(*ns));
        }
    }

    #[test]
    fn untranslated_entries_carry_null_sentinel() {
        // The packed DB stores the literal "null" for untranslated tags;
        // TagDb::translate is what filters it to None.
        let db = db(&[("f:not_translated", "null"), ("f:yuri", "百合")]);
        assert_eq!(db.translate("f:yuri").as_deref(), Some("百合"));
        assert_eq!(db.translate("f:not_translated").as_deref(), Some("null"));
        assert_eq!(db.translate("f:missing"), None);
    }
}
