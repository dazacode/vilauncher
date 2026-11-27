use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub ray_tracing: bool,
    pub probe_rays: bool,
    pub hooks: bool,
}

pub const RAY_TRACING: &[&str] = &["VICTORY_BVH_BUILDER"];
pub const PROBE_RAYS: &[&str] = &["VICTORY_ACTIVATE_RAYS", "VICTORY_RT_RESOLVE"];
pub const HOOKS: &[&str] = &["VICTORY_GPU_ONLY", "VICTORY_KNOWN_WRITES"];

/// Read a literal launch hashtable and recognize the supported Scarlett flag mappings.
pub fn parse(script: &str) -> Result<(String, Options), String> {
    let lines: Vec<_> = script
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .collect();
    let block = lines
        .iter()
        .enumerate()
        .find_map(|(index, line)| {
            let (variable, value) = line.split_once('=')?;
            let variable = variable.trim();
            if matches!(variable, "$set" | "$settings")
                && matches!(value.trim(), "@{" | "[ordered]@{")
            {
                Some((index + 1, variable))
            } else {
                None
            }
        })
        .ok_or("Expected a $set or $settings PowerShell launch hashtable.")?;
    let (start, variable) = block;
    let end = lines[start..]
        .iter()
        .position(|line| *line == "}")
        .ok_or("Could not find the end of the launch hashtable.")?
        + start;
    let defaults: BTreeMap<_, _> = lines[start..end]
        .iter()
        .flat_map(|line| assignments(line))
        .filter_map(|assignment| literal_assignment(assignment, None))
        .filter(|(key, _)| {
            !matches!(
                *key,
                "VICTORY_GAME_DIR" | "VICTORY_CACHE_DIR" | "VICTORY_DATA_DIR" | "VICTORY_CONFIG"
            )
        })
        .collect();
    if defaults.is_empty() {
        return Err("The script has no supported literal VICTORY_* launch defaults.".into());
    }
    let prefix = format!("{variable}.");
    let after = &lines[end + 1..];
    let options = Options {
        ray_tracing: has_flag(after, &prefix, &["if($raytracing)"], RAY_TRACING),
        probe_rays: has_flag(
            after,
            &prefix,
            &["if($proberays-and-not$raytracing)"],
            PROBE_RAYS,
        ),
        hooks: has_flag(
            after,
            &prefix,
            &["if($hooks-and-not$nohooks)", "if($hooks)"],
            HOOKS,
        ),
    };
    let switches = defaults
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    Ok((switches, options))
}

fn assignments(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut quote = None;
    let mut start = 0;
    for (index, ch) in line.char_indices() {
        match (quote, ch) {
            (Some(open), ch) if open == ch => quote = None,
            (None, '\'' | '"') => quote = Some(ch),
            (None, ';') => {
                parts.push(&line[start..index]);
                start = index + 1;
            }
            (None, '#') => {
                parts.push(&line[start..index]);
                return parts;
            }
            _ => {}
        }
    }
    parts.push(&line[start..]);
    parts
}

fn literal_assignment<'a>(assignment: &'a str, prefix: Option<&str>) -> Option<(&'a str, &'a str)> {
    let (key, value) = assignment.trim().split_once('=')?;
    let key = key.trim();
    let key = prefix.map_or(Some(key), |prefix| key.strip_prefix(prefix))?;
    if !key.starts_with("VICTORY_")
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return None;
    }
    let value = value.trim();
    let quote = value.chars().next()?;
    if !matches!(quote, '\'' | '"') {
        return None;
    }
    let (literal, tail) = value[1..].split_once(quote)?;
    let tail = tail.trim();
    if (!tail.is_empty() && !tail.starts_with('#'))
        || literal
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '$' | '`'))
    {
        return None;
    }
    Some((key, literal))
}

fn has_flag(lines: &[&str], prefix: &str, guards: &[&str], keys: &[&str]) -> bool {
    lines.iter().enumerate().any(|(index, line)| {
        let Some((guard, first)) = line.split_once('{') else {
            return false;
        };
        let guard = guard
            .split_whitespace()
            .collect::<String>()
            .to_ascii_lowercase();
        if !guards.contains(&guard.as_str()) {
            return false;
        }
        let mut values = BTreeMap::new();
        for part in std::iter::once(first).chain(lines[index + 1..].iter().copied()) {
            let (content, end) = part
                .split_once('}')
                .map_or((part, false), |(before, _)| (before, true));
            if !content.trim_start().starts_with('#') {
                values.extend(
                    assignments(content)
                        .into_iter()
                        .filter_map(|value| literal_assignment(value, Some(prefix))),
                );
            }
            if end {
                return keys.iter().all(|key| values.get(key) == Some(&"1"));
            }
        }
        false
    })
}

pub fn enabled(switches: &str, keys: &[&str]) -> bool {
    keys.iter().all(|key| {
        switches
            .lines()
            .rev()
            .filter_map(|line| line.split_once('='))
            .find(|(name, _)| name.trim() == *key)
            .is_some_and(|(_, value)| value.trim() == "1")
    })
}

pub fn set_enabled(switches: &mut String, keys: &[&str], enabled: bool) {
    let mut lines: Vec<_> = switches
        .lines()
        .filter(|line| {
            !line
                .split_once('=')
                .is_some_and(|(key, _)| keys.contains(&key.trim()))
        })
        .map(str::to_owned)
        .collect();
    lines.extend(
        keys.iter()
            .map(|key| format!("{key}={}", u8::from(enabled))),
    );
    *switches = lines.join("\n");
}

#[cfg(test)]
#[test]
fn reads_scarlett_style_defaults_and_recognizes_supported_flags() {
    let script = r#"
$set = [ordered]@{
    VICTORY_GAME_DIR = $Game
    VICTORY_CACHE_DIR = 'C:/Author/cache'
    VICTORY_JOIN_COPIES = '1'; VICTORY_CARD_AWAKE = '0'
    VICTORY_VALUE = 'a;b#c' # comment; VICTORY_IGNORED = '1'
    VICTORY_DYNAMIC = "$value"
}
if ($Hooks -and -not $NoHooks) {
    $set.VICTORY_GPU_ONLY = '1'; $set.VICTORY_KNOWN_WRITES = '1'
}
if ($ProbeRays -and -not $RayTracing) { $set.VICTORY_ACTIVATE_RAYS = '1'; $set.VICTORY_RT_RESOLVE = '1' }
if ($RayTracing) { $set.VICTORY_BVH_BUILDER = '1' }
if ($AutoStart) { $set.VICTORY_AUTO_INPUT = '1' }
"#;
    let (mut switches, options) = parse(script).unwrap();
    assert_eq!(
        options,
        Options {
            ray_tracing: true,
            probe_rays: true,
            hooks: true
        }
    );
    assert_eq!(
        switches,
        "VICTORY_CARD_AWAKE=0\nVICTORY_JOIN_COPIES=1\nVICTORY_VALUE=a;b#c"
    );
    set_enabled(&mut switches, PROBE_RAYS, true);
    assert!(enabled(&switches, PROBE_RAYS));
    set_enabled(&mut switches, PROBE_RAYS, false);
    assert!(!enabled(&switches, PROBE_RAYS));
    assert_eq!(switches.matches("VICTORY_RT_RESOLVE=").count(), 1);
    assert_eq!(
        parse("$settings = @{\nVICTORY_LOG = 'warn'\n}").unwrap().1,
        Options::default()
    );
    assert!(parse("Write-Host 'No launch hashtable'").is_err());
}
