//! The memory a project's author says it uses, read from the tracker's
//! words: "about 30 GB of RAM", "12 GB of video memory", "16GB VRAM".
//! Amounts that describe the test PC ("Platform: ..., 64 GB RAM"), a
//! recommendation, a card or a fix ("8 GB graphics card fix") are left out.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Reported {
    pub ram_gb: Option<f64>,
    pub vram_gb: Option<f64>,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Ram,
    Vram,
}

/// Words after the amount that say which memory, longest first.
const KINDS: &[(&str, Kind)] = &[
    ("dedicated video memory", Kind::Vram),
    ("dedicated vram", Kind::Vram),
    ("video memory", Kind::Vram),
    ("video ram", Kind::Vram),
    ("gpu memory", Kind::Vram),
    ("graphics memory", Kind::Vram),
    ("vram", Kind::Vram),
    ("system memory", Kind::Ram),
    ("system ram", Kind::Ram),
    ("ram", Kind::Ram),
    ("memory", Kind::Ram),
];

/// A word right after the memory that makes it something else: "16 GB VRAM
/// card", "8 GB VRAM fix".
const NOT_USE: &[&str] = &["card", "cards", "gpu", "gpus", "fix", "mode", "option", "patch", "setting", "toggle", "kit", "stick", "sticks", "module", "modules"];

/// Words that mark a description of the test PC.
const SPEC: &[&str] = &["platform", "tested on", "test pc", "test rig", "test system", "specs", "ryzen", "core i", "windows 1"];

/// Words that say what the project uses.
const USE: &[&str] = &["use", "uses", "using", "used", "needs", "need", "needed", "requires", "require", "required", "takes", "consumes", "peaks", "fills", "eats"];

/// The most RAM and video memory the text says the project uses.
pub(crate) fn parse<'a>(texts: impl IntoIterator<Item = &'a str>) -> Reported {
    let mut out = Reported::default();
    for text in texts {
        for clause in clauses(text) {
            for (gb, kind) in amounts(&clause) {
                let slot = if kind == Kind::Vram { &mut out.vram_gb } else { &mut out.ram_gb };
                *slot = Some(slot.map_or(gb, |old| old.max(gb)));
            }
        }
    }
    out
}

/// Sentences and parts split at ";", lower case.
fn clauses(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = lower.chars().peekable();
    while let Some(c) = chars.next() {
        let ends = match c {
            ';' | '\n' => true,
            // "1.3 GB" stays together; "fps. It" does not.
            '.' | '!' | '?' => chars.peek().map_or(true, |n| n.is_whitespace()),
            _ => false,
        };
        if ends {
            out.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    out.push(current);
    out.retain(|c| !c.trim().is_empty());
    out
}

fn word_at(chars: &[char], i: usize) -> bool {
    i == 0 || !chars[i - 1].is_alphanumeric()
}

fn ends_word(chars: &[char], i: usize) -> bool {
    chars.get(i).map_or(true, |c| !c.is_alphanumeric())
}

fn starts_with(chars: &[char], i: usize, s: &str) -> bool {
    let mut j = i;
    for c in s.chars() {
        if chars.get(j) != Some(&c) {
            return false;
        }
        j += 1;
    }
    true
}

fn skip_spaces(chars: &[char], mut i: usize) -> usize {
    while chars.get(i).is_some_and(|c| c.is_whitespace()) {
        i += 1;
    }
    i
}

/// A number like "30" or "1.5" at `i`: its value and where it ends.
fn number(chars: &[char], i: usize) -> Option<(f64, usize)> {
    let mut j = i;
    while chars.get(j).is_some_and(|c| c.is_ascii_digit()) {
        j += 1;
    }
    if j == i {
        return None;
    }
    if chars.get(j) == Some(&'.') && chars.get(j + 1).is_some_and(|c| c.is_ascii_digit()) {
        j += 1;
        while chars.get(j).is_some_and(|c| c.is_ascii_digit()) {
            j += 1;
        }
    }
    let text: String = chars[i..j].iter().collect();
    Some((text.parse().ok()?, j))
}

/// The memory amounts in one clause, in GB. A range counts its top.
fn amounts(clause: &str) -> Vec<(f64, Kind)> {
    let chars: Vec<char> = clause.chars().collect();
    // In a description of the test PC only what comes after "uses" counts.
    let spec = (0..chars.len()).any(|i| word_at(&chars, i) && SPEC.iter().any(|w| starts_with(&chars, i, w)));
    let from = if spec {
        match first_word(&chars, USE) {
            Some(at) => at,
            None => return Vec::new(),
        }
    } else {
        0
    };
    let mut out = Vec::new();
    let mut i = from;
    while i < chars.len() {
        if !(chars[i].is_ascii_digit() && word_at(&chars, i) && (i == 0 || chars[i - 1] != '.')) {
            i += 1;
            continue;
        }
        let Some((mut gb, mut j)) = number(&chars, i) else {
            i += 1;
            continue;
        };
        // "40-46 GB", "40 to 46 GB".
        let k = skip_spaces(&chars, j);
        let after_dash = if matches!(chars.get(k), Some('-' | '\u{2013}' | '\u{2014}')) {
            Some(k + 1)
        } else if starts_with(&chars, k, "to ") {
            Some(k + 2)
        } else {
            None
        };
        if let Some(next) = after_dash.and_then(|a| number(&chars, skip_spaces(&chars, a))) {
            gb = gb.max(next.0);
            j = next.1;
        }
        i = j;
        let mut k = skip_spaces(&chars, j);
        let unit = ["gib", "gb"].into_iter().find(|u| starts_with(&chars, k, u) && ends_word(&chars, k + u.len()));
        let Some(unit) = unit else { continue };
        k = skip_spaces(&chars, k + unit.len());
        if starts_with(&chars, k, "of ") {
            k = skip_spaces(&chars, k + 3);
        }
        let Some((words, kind)) = KINDS.iter().find(|(w, _)| starts_with(&chars, k, w) && ends_word(&chars, k + w.len())) else {
            continue;
        };
        let rest: String = chars[k + words.len()..].iter().collect();
        let mut next_words = rest.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty());
        let next = next_words.next().unwrap_or("");
        if NOT_USE.contains(&next) {
            continue;
        }
        // "64 GB of RAM recommended", "(64 GB of RAM or more recommended)".
        if rest.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).take(4).any(|w| w.starts_with("recommend")) {
            continue;
        }
        if gb > 0.0 && gb <= 1024.0 {
            out.push(((gb * 10.0).round() / 10.0, *kind));
        }
    }
    out
}

/// Where the first of `words` starts as a whole word.
fn first_word(chars: &[char], words: &[&str]) -> Option<usize> {
    (0..chars.len()).find(|&i| word_at(chars, i) && words.iter().any(|w| starts_with(chars, i, w) && ends_word(chars, i + w.chars().count())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> (Option<f64>, Option<f64>) {
        let r = parse([text]);
        (r.ram_gb, r.vram_gb)
    }

    #[test]
    fn reported_memory_is_read_in_its_usual_phrasings() {
        assert_eq!(one("Very playable: about 22 fps on average and no crashes, using about 30 GB of RAM and 12 GB of video memory"), (Some(30.0), Some(12.0)));
        assert_eq!(one("It averages 25 fps with 8 GB system RAM and 16 GB VRAM."), (Some(8.0), Some(16.0)));
        assert_eq!(one("Runs fine on 16GB VRAM."), (None, Some(16.0)));
        assert_eq!(one("It uses 24 GB of system memory at peak."), (Some(24.0), None));
        assert_eq!(one("Needs 30 GB of system memory"), (Some(30.0), None));
        assert_eq!(one("Peaks at 10.5 GB of GPU memory"), (None, Some(10.5)));
        assert_eq!(one("Uses 40-46 GB of memory (64 GB of RAM recommended)"), (Some(46.0), None));
        assert_eq!(one("Uses 20 to 22 GiB of RAM"), (Some(22.0), None));
        assert_eq!(one("needs 12GB of dedicated VRAM"), (None, Some(12.0)));
    }

    #[test]
    fn other_amounts_are_not_taken_for_its_memory() {
        for text in [
            "Ships a 1.3 GB shader cache.",
            "Adds an 8 GB graphics card fix.",
            "Enable the 8 GB VRAM fix for smaller cards",
            "Platform: AMD Radeon RX 9070 XT 16 GB, Ryzen 7 9800X3D, 64 GB RAM, Windows 11",
            "Tested on an RTX 4090 with 64 GB RAM",
            "Lists more video memory than a 16 GB card has",
            "Works on a 24 GB VRAM card",
            "64 GB of RAM recommended",
            "A 2 GB download, 5-7 fps, version 1.12 GB",
            "Its README still reports 5-7 fps",
        ] {
            assert_eq!(one(text), (None, None), "{text}");
        }
    }

    #[test]
    fn the_test_pc_only_counts_after_it_says_what_it_uses() {
        assert_eq!(one("Tested on a PC with 64 GB RAM, it uses 20 GB of VRAM"), (None, Some(20.0)));
    }

    #[test]
    fn the_biggest_amount_across_summary_and_lists_wins() {
        let r = parse(["Uses about 30 GB of RAM. Later builds need 34 GB of RAM.", "Uses 12 GB of video memory", "Platform: 64 GB RAM"]);
        assert_eq!(r, Reported { ram_gb: Some(34.0), vram_gb: Some(12.0) });
        assert_eq!(parse([""]), Reported::default());
    }
}
