//! The launcher's own messages in the player's language.
//!
//! A message for the player is written in English where it is made, and
//! marked: `msg!` is `format!` for a message (`msg!("Could not open {path}")`),
//! `msg_str!` a `&'static str` message without placeholders. Inside the
//! launcher it stays English, so logs, problem reports, `--fix` and the agent
//! tool read the same as before. It is translated only where it crosses into
//! the window, with [`localize`]: the snapshot, an action's answer and its
//! errors.
//!
//! [`localize`] finds the English text in the catalog of the current
//! language: as it is, or as a message with placeholders whose values are
//! taken from the text (and translated in turn when they are messages too).
//! Text it does not know (a path, Git's output, an OS error) stays as it is.
//!
//! Catalogs live in `locales/<language>/*.json`: objects from the English
//! message, with its placeholders as `{name}` (format specs dropped, `{}` as
//! `{0}`, `{1}`…), to the translation. The tests check that every marked
//! message has a translation in every catalog with the same placeholders, and
//! that no catalog keeps a message the code no longer has. docs/I18N.md says
//! how to add a message or a language.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// An English message for the player, made as `format!` makes it. The window
/// shows it translated (see the module notes); everywhere else it stays English.
macro_rules! msg {
    ($($arg:tt)*) => {
        ::std::format!($($arg)*)
    };
}

/// An English message for the player without placeholders, as a `&'static str`.
macro_rules! msg_str {
    ($text:literal) => {
        $text
    };
}

/// The languages the launcher speaks. English is the source: it has no catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Locale {
    En,
    ZhTw,
    ZhCn,
}

impl Locale {
    pub const ALL: [Locale; 3] = [Locale::En, Locale::ZhTw, Locale::ZhCn];

    /// The language's BCP 47 tag, as the window and settings.json use it.
    pub fn tag(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::ZhTw => "zh-TW",
            Locale::ZhCn => "zh-CN",
        }
    }

    /// One of the tags [`Locale::tag`] gives (any case); none for others.
    pub fn from_tag(tag: &str) -> Option<Locale> {
        Locale::ALL
            .into_iter()
            .find(|l| l.tag().eq_ignore_ascii_case(tag.trim()))
    }

    /// The closest language for any BCP 47 tag, such as `zh-Hant-HK` or
    /// `en-GB`: traditional Chinese for Taiwan, Hong Kong, Macau and `Hant`,
    /// simplified for the rest of Chinese. None when the launcher doesn't speak it.
    pub fn matching(tag: &str) -> Option<Locale> {
        let tag = tag.trim().to_ascii_lowercase().replace('_', "-");
        let mut parts = tag.split('-');
        match parts.next()? {
            "en" => Some(Locale::En),
            "zh" => {
                let traditional = parts.any(|p| matches!(p, "hant" | "tw" | "hk" | "mo"));
                Some(if traditional {
                    Locale::ZhTw
                } else {
                    Locale::ZhCn
                })
            }
            _ => None,
        }
    }
}

/// The player's choice (a tag; none: follow the system), else the first of
/// the system's languages the launcher speaks, else English.
pub fn resolve(choice: Option<&str>, system: &[String]) -> Locale {
    choice
        .and_then(Locale::from_tag)
        .or_else(|| system.iter().find_map(|tag| Locale::matching(tag)))
        .unwrap_or(Locale::En)
}

static CURRENT: RwLock<Locale> = RwLock::new(Locale::En);

/// The language the window is shown in.
pub fn current() -> Locale {
    *CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

pub fn set_current(locale: Locale) {
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = locale;
}

/// `text`, an English message, in the current language. Text that is not a
/// known message comes back as it is.
pub fn localize(text: &str) -> String {
    match catalog(current()) {
        Some(catalog) => catalog.localize(text, 0),
        None => text.to_owned(),
    }
}

/// [`localize`] for an optional message.
pub fn localize_opt(text: Option<&str>) -> Option<String> {
    text.map(localize)
}

/// The catalog files, per language. Each area of the code keeps its own file
/// so that changes in one area don't touch the others.
const CATALOG_FILES: &[(Locale, &str, &str)] = &[
    (
        Locale::ZhTw,
        "zh-TW/core.json",
        include_str!("../locales/zh-TW/core.json"),
    ),
    (
        Locale::ZhTw,
        "zh-TW/launcher.json",
        include_str!("../locales/zh-TW/launcher.json"),
    ),
    (
        Locale::ZhTw,
        "zh-TW/services.json",
        include_str!("../locales/zh-TW/services.json"),
    ),
    (
        Locale::ZhCn,
        "zh-CN/core.json",
        include_str!("../locales/zh-CN/core.json"),
    ),
    (
        Locale::ZhCn,
        "zh-CN/launcher.json",
        include_str!("../locales/zh-CN/launcher.json"),
    ),
    (
        Locale::ZhCn,
        "zh-CN/services.json",
        include_str!("../locales/zh-CN/services.json"),
    ),
];

fn catalog(locale: Locale) -> Option<&'static Catalog> {
    static CATALOGS: OnceLock<HashMap<Locale, Catalog>> = OnceLock::new();
    CATALOGS
        .get_or_init(|| {
            let mut all: HashMap<Locale, Vec<(String, String)>> = HashMap::new();
            for (locale, _, text) in CATALOG_FILES {
                // A broken file is caught by the tests; at run time it only means English.
                let entries: HashMap<String, String> =
                    serde_json::from_str(text).unwrap_or_default();
                all.entry(*locale).or_default().extend(entries);
            }
            all.into_iter()
                .map(|(locale, entries)| (locale, Catalog::new(entries)))
                .collect()
        })
        .get(&locale)
}

/// A piece of a message: text, or a placeholder's name.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Piece {
    Text(String),
    Slot(String),
}

/// `text` in pieces. `{{` and `}}` are a brace; `{name}` and `{name:spec}` a
/// placeholder (the spec dropped); `{}` the next numbered one. None when a
/// brace is not closed.
fn pieces(text: &str) -> Option<Vec<Piece>> {
    let mut out = Vec::new();
    let mut literal = String::new();
    let mut next_index = 0;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                literal.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                literal.push('}');
            }
            '{' => {
                let mut inside = String::new();
                loop {
                    match chars.next()? {
                        '}' => break,
                        c => inside.push(c),
                    }
                }
                let name = inside.split(':').next().unwrap_or("").trim().to_owned();
                let name = if name.is_empty() {
                    next_index += 1;
                    (next_index - 1).to_string()
                } else {
                    name
                };
                if !literal.is_empty() {
                    out.push(Piece::Text(std::mem::take(&mut literal)));
                }
                out.push(Piece::Slot(name));
            }
            c => literal.push(c),
        }
    }
    if !literal.is_empty() {
        out.push(Piece::Text(literal));
    }
    Some(out)
}

/// A message in the catalog's own spelling: placeholders as `{name}`, braces doubled.
#[cfg(test)]
fn normalize(text: &str) -> Option<String> {
    Some(
        pieces(text)?
            .iter()
            .map(|piece| match piece {
                Piece::Text(t) => t.replace('{', "{{").replace('}', "}}"),
                Piece::Slot(name) => format!("{{{name}}}"),
            })
            .collect(),
    )
}

struct Catalog {
    /// Messages without placeholders: English text to translation.
    exact: HashMap<String, String>,
    /// Messages with placeholders: the English pieces and the translation's pieces,
    /// the most specific (longest fixed text) first.
    templates: Vec<(Vec<Piece>, Vec<Piece>)>,
}

impl Catalog {
    fn new(entries: impl IntoIterator<Item = (String, String)>) -> Catalog {
        let mut exact = HashMap::new();
        let mut templates = Vec::new();
        for (english, translated) in entries {
            let (Some(from), Some(to)) = (pieces(&english), pieces(&translated)) else {
                continue;
            };
            if from.iter().any(|p| matches!(p, Piece::Slot(_))) {
                templates.push((from, to));
            } else {
                exact.insert(text_of(&from), text_of(&to));
            }
        }
        templates.sort_by_key(|(from, _)| std::cmp::Reverse(fixed_len(from)));
        Catalog { exact, templates }
    }

    fn localize(&self, text: &str, depth: u8) -> String {
        if let Some((_, found)) = self.best(text, depth) {
            return found;
        }
        // Several messages, one per line (errors are sometimes joined that way).
        if text.contains('\n') && depth < 4 {
            return text
                .split('\n')
                .map(|line| self.localize(line, depth + 1))
                .collect::<Vec<_>>()
                .join("\n");
        }
        text.to_owned()
    }

    /// The best reading of `text` as a message, and its translation. A value
    /// can be a message too ("Install failed: {error}"), so a reading scores
    /// the fixed text it explains, its values' messages included; on a tie,
    /// a message that starts with fixed text wins ("Updated {when}" over
    /// "{days} days ago" for "Updated 3 days ago").
    fn best(&self, text: &str, depth: u8) -> Option<(usize, String)> {
        if let Some(found) = self.exact.get(text) {
            return Some((text.len(), found.clone()));
        }
        let mut best: Option<((usize, bool), String)> = None;
        for (from, to) in &self.templates {
            let mut values = HashMap::new();
            if !matches(from, text, &mut values) {
                continue;
            }
            let mut score = fixed_len(from);
            let mut filled: HashMap<&str, String> = HashMap::new();
            for (name, value) in &values {
                let read = (depth < 4).then(|| self.best(value, depth + 1)).flatten();
                let (value_score, translated) = read.unwrap_or((0, (*value).to_owned()));
                score += value_score;
                filled.insert(name.as_str(), translated);
            }
            let rank = (score, matches!(from.first(), Some(Piece::Text(_))));
            if best.as_ref().is_some_and(|(b, _)| *b >= rank) {
                continue;
            }
            let out = to
                .iter()
                .map(|piece| match piece {
                    Piece::Text(t) => t.clone(),
                    Piece::Slot(name) => filled
                        .get(name.as_str())
                        .cloned()
                        .unwrap_or_else(|| format!("{{{name}}}")),
                })
                .collect();
            best = Some((rank, out));
        }
        best.map(|((score, _), out)| (score, out))
    }
}

fn text_of(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Text(t) => t.clone(),
            Piece::Slot(name) => format!("{{{name}}}"),
        })
        .collect()
}

fn fixed_len(pieces: &[Piece]) -> usize {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Text(t) => t.len(),
            Piece::Slot(_) => 0,
        })
        .sum()
}

/// Whether `text` is the message `pattern` with some values in its
/// placeholders, which are put in `values`. A placeholder takes as little as
/// it can, and more when the rest doesn't fit otherwise.
fn matches<'t>(pattern: &[Piece], text: &'t str, values: &mut HashMap<String, &'t str>) -> bool {
    match pattern {
        [] => text.is_empty(),
        [Piece::Text(t), rest @ ..] => {
            text.starts_with(t.as_str()) && matches(rest, &text[t.len()..], values)
        }
        [Piece::Slot(name)] => {
            values.insert(name.clone(), text);
            true
        }
        [Piece::Slot(name), Piece::Text(next), ..] => {
            let mut from = 0;
            while let Some(at) = text[from..].find(next.as_str()) {
                let end = from + at;
                let mut tried = values.clone();
                tried.insert(name.clone(), &text[..end]);
                if matches(&pattern[1..], &text[end..], &mut tried) {
                    *values = tried;
                    return true;
                }
                // Next occurrence: step past this one's first character.
                from = end + text[end..].chars().next().map_or(1, char::len_utf8);
            }
            false
        }
        // Two placeholders side by side can't be told apart; the tests refuse them.
        [Piece::Slot(_), Piece::Slot(_), ..] => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    fn catalog_of(entries: &[(&str, &str)]) -> Catalog {
        Catalog::new(entries.iter().map(|(a, b)| (a.to_string(), b.to_string())))
    }

    #[test]
    fn languages_follow_the_choice_then_the_system() {
        assert_eq!(resolve(Some("zh-TW"), &["en-US".into()]), Locale::ZhTw);
        assert_eq!(resolve(Some("zh-cn"), &[]), Locale::ZhCn);
        assert_eq!(
            resolve(None, &["fr-FR".into(), "zh-Hant-HK".into()]),
            Locale::ZhTw
        );
        assert_eq!(resolve(None, &["zh-SG".into()]), Locale::ZhCn);
        assert_eq!(resolve(None, &["zh".into()]), Locale::ZhCn);
        assert_eq!(resolve(None, &["zh_TW".into()]), Locale::ZhTw);
        assert_eq!(resolve(Some("klingon"), &["de".into()]), Locale::En);
        assert_eq!(resolve(None, &[]), Locale::En);
    }

    #[test]
    fn messages_are_found_with_their_values() {
        let c = catalog_of(&[
            ("Stopped", "已停止"),
            ("Could not open {path}: {error}", "無法開啟 {path}：{error}"),
            ("Install failed: {error}", "安裝失敗：{error}"),
            ("Updated {when}", "{when}更新"),
            ("{days} days ago", "{days} 天前"),
            ("{{literal}} {name}", "{{字面}} {name}"),
        ]);
        assert_eq!(c.localize("Stopped", 0), "已停止");
        assert_eq!(c.localize("Unknown text", 0), "Unknown text");
        // A placeholder takes as little as it can: the path stops at the first ": ".
        assert_eq!(
            c.localize(
                "Could not open C:\\a\\c.txt: os error 5: Access is denied.",
                0
            ),
            "無法開啟 C:\\a\\c.txt：os error 5: Access is denied."
        );
        assert_eq!(c.localize("Install failed: Stopped", 0), "安裝失敗：已停止");
        assert_eq!(c.localize("Updated 3 days ago", 0), "3 天前更新");
        assert_eq!(c.localize("{literal} x", 0), "{字面} x");
        assert_eq!(
            c.localize("Stopped\nInstall failed: x", 0),
            "已停止\n安裝失敗：x"
        );
    }

    #[test]
    fn placeholders_are_spelled_one_way() {
        assert_eq!(
            normalize("a {size:.1} b {} {} {name:?} {{x}}").unwrap(),
            "a {size} b {0} {1} {name} {{x}}"
        );
        assert!(pieces("open {brace").is_none());
    }

    #[test]
    fn english_messages_stay_english_without_a_catalog() {
        assert!(catalog(Locale::En).is_none());
    }

    // --- The catalogs against the code ---------------------------------------

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// The text of the Rust string literal at the start of `code` (plain or
    /// raw), and the rest of the code after it. None when `code` doesn't start with one.
    fn string_literal(code: &str) -> Option<(String, &str)> {
        let code = code.trim_start();
        if let Some(raw) = code.strip_prefix('r') {
            let hashes = raw.chars().take_while(|c| *c == '#').count();
            let body = raw[hashes..].strip_prefix('"')?;
            let end = format!("\"{}", "#".repeat(hashes));
            let at = body.find(&end)?;
            return Some((body[..at].to_owned(), &body[at + end.len()..]));
        }
        let body = code.strip_prefix('"')?;
        let mut out = String::new();
        let mut chars = body.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => return Some((out, &body[i + 1..])),
                '\\' => match chars.next()?.1 {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    '0' => out.push('\0'),
                    '\\' => out.push('\\'),
                    '"' => out.push('"'),
                    '\'' => out.push('\''),
                    'u' => {
                        let mut hex = String::new();
                        for (_, c) in chars.by_ref() {
                            match c {
                                '{' => {}
                                '}' => break,
                                c => hex.push(c),
                            }
                        }
                        out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                    }
                    '\n' | '\r' => {
                        // A line continuation: the line break and the next line's indent go.
                        let rest = &body[i + 2..];
                        let skip = rest.len() - rest.trim_start().len();
                        for _ in 0..rest[..skip].chars().count() {
                            chars.next();
                        }
                    }
                    other => panic!("unknown escape \\{other} in a message"),
                },
                c => out.push(c),
            }
        }
        None
    }

    /// Every marked message in the code, normalized, with the file it is in.
    fn marked_messages() -> BTreeMap<String, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&root, &mut files);
        let mut found = BTreeMap::new();
        for file in files {
            // This file's own examples are not messages.
            if file.file_name().is_some_and(|n| n == "i18n.rs") {
                continue;
            }
            let code = std::fs::read_to_string(&file).unwrap();
            let name = file.strip_prefix(&root).unwrap().display().to_string();
            for marker in ["msg!(", "msg_str!("] {
                let mut rest = code.as_str();
                while let Some(at) = rest.find(marker) {
                    // Skip `some_msg!(`: the marker must start a word.
                    let before = rest[..at].chars().next_back();
                    rest = &rest[at + marker.len()..];
                    if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                        continue;
                    }
                    let (text, after) = string_literal(rest).unwrap_or_else(|| {
                        panic!("{name}: {marker} must start with a string literal")
                    });
                    let normalized = normalize(&text)
                        .unwrap_or_else(|| panic!("{name}: unclosed brace in {text:?}"));
                    found.insert(normalized, name.clone());
                    rest = after;
                }
            }
        }
        // The shipped known problems' summaries reach the window inside a message
        // ("Known problem reported for this project: {summary}"), so each is a message too.
        let manifest =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("known-problems/known-problems.json");
        let text = std::fs::read_to_string(&manifest).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        for entry in json["entries"].as_array().into_iter().flatten() {
            let summary = entry["summary"].as_str().unwrap_or_default();
            let normalized = normalize(summary)
                .unwrap_or_else(|| panic!("known-problems.json: unclosed brace in {summary:?}"));
            found.insert(normalized, "known-problems/known-problems.json".to_owned());
        }
        // The shipped add-ons' name, summary and warning reach the window translated
        // (the Launch tab's add-ons), so each is a message too.
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("addons");
        let mut manifests: Vec<PathBuf> = std::fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "json"))
            .collect();
        manifests.sort();
        for manifest in manifests {
            let name = format!("addons/{}", manifest.file_name().unwrap().to_string_lossy());
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
            for field in ["name", "summary", "warning"] {
                let text = json[field].as_str().unwrap_or_default();
                let normalized =
                    normalize(text).unwrap_or_else(|| panic!("{name}: unclosed brace in {text:?}"));
                found.insert(normalized, name.clone());
            }
        }
        found
    }

    fn placeholders(text: &str) -> BTreeSet<String> {
        pieces(text)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| match p {
                Piece::Slot(name) => Some(name),
                Piece::Text(_) => None,
            })
            .collect()
    }

    #[test]
    fn every_message_is_translated_in_every_language() {
        let messages = marked_messages();
        let mut problems = Vec::new();
        for (message, file) in &messages {
            let p = pieces(message).unwrap();
            if p.windows(2)
                .any(|w| matches!(w, [Piece::Slot(_), Piece::Slot(_)]))
            {
                problems.push(format!(
                    "{file}: two placeholders side by side in {message:?}"
                ));
            }
            if matches!(p.as_slice(), [Piece::Slot(_)]) {
                problems.push(format!(
                    "{file}: {message:?} is only a placeholder, not a message"
                ));
            }
        }
        for locale in Locale::ALL.into_iter().filter(|l| *l != Locale::En) {
            let mut merged: BTreeMap<String, (String, &str)> = BTreeMap::new();
            for (file_locale, file, text) in CATALOG_FILES {
                if *file_locale != locale {
                    continue;
                }
                let entries: BTreeMap<String, String> =
                    serde_json::from_str(text).unwrap_or_else(|e| {
                        panic!("locales/{file} is not a JSON object of strings: {e}")
                    });
                for (english, translated) in entries {
                    let Some(key) = normalize(&english) else {
                        problems.push(format!("locales/{file}: unclosed brace in key {english:?}"));
                        continue;
                    };
                    if key != english {
                        problems.push(format!("locales/{file}: write {english:?} as {key:?}"));
                    }
                    if translated.trim().is_empty() {
                        problems.push(format!("locales/{file}: {english:?} has no translation"));
                    }
                    if placeholders(&english) != placeholders(&translated) {
                        problems.push(format!(
                            "locales/{file}: {english:?} and {translated:?} have different placeholders"
                        ));
                    }
                    if let Some((other, other_file)) = merged.get(&key) {
                        if *other != translated {
                            problems.push(format!(
                                "{english:?} is translated twice, differently, in locales/{other_file} and locales/{file}"
                            ));
                        }
                    }
                    merged.insert(key, (translated, file));
                }
            }
            for (message, file) in &messages {
                if !merged.contains_key(message) {
                    problems.push(format!(
                        "{}: {file}: no translation for {message:?}",
                        locale.tag()
                    ));
                }
            }
            for (key, (_, file)) in &merged {
                if !messages.contains_key(key) {
                    problems.push(format!(
                        "locales/{file}: {key:?} is no longer a message in the code"
                    ));
                }
            }
        }
        assert!(
            problems.is_empty(),
            "{} catalog problems:\n{}",
            problems.len(),
            problems.join("\n")
        );
    }
}
