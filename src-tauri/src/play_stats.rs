use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

type Records = BTreeMap<String, Record>;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(from = "Records", into = "Records")]
pub struct History(Arc<Mutex<Records>>);

#[derive(Clone, Default, Serialize, Deserialize)]
struct Record {
    total_ms: u64,
    session_ms: u64,
    samples: u64,
    mean_fps: f64,
    latest_fps: Option<f64>,
    #[serde(default)]
    all_sessions: Option<FpsAverage>,
    #[serde(default)]
    without_exact_60: FpsAverage,
    #[serde(skip)]
    started: Option<Instant>,
}

impl Record {
    fn average(&self) -> FpsAverage {
        // Older versions saved only the most recent session and its sample count.
        self.all_sessions.unwrap_or(FpsAverage {
            samples: self.samples,
            mean: self.mean_fps,
        })
    }

    fn snapshot(&self, now: Instant) -> Self {
        let mut record = self.clone();
        if let Some(started) = record.started.take() {
            let ms = now.saturating_duration_since(started).as_millis() as u64;
            record.total_ms = record.total_ms.saturating_add(ms);
            record.session_ms = ms;
        }
        record
    }

    fn selected_average(&self, include_exact_60: bool) -> Option<f64> {
        let avg = if include_exact_60 {
            self.average()
        } else {
            self.without_exact_60
        };
        (avg.samples > 0).then_some(avg.mean)
    }
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
struct FpsAverage {
    samples: u64,
    mean: f64,
}

impl FpsAverage {
    fn observe(&mut self, value: f64) {
        self.samples += 1;
        self.mean += (value - self.mean) / self.samples as f64;
    }
}

pub struct ProfileStats {
    pub playtime: String,
    pub average_fps: Option<f64>,
    #[cfg(test)]
    pub latest_fps: Option<f64>,
    #[cfg(test)]
    pub playing: bool,
}

impl From<Records> for History {
    fn from(records: Records) -> Self {
        Self(Arc::new(Mutex::new(records)))
    }
}

impl From<History> for Records {
    fn from(history: History) -> Self {
        let records = history.0.lock().unwrap();
        let now = Instant::now();
        records
            .iter()
            .map(|(key, record)| (key.clone(), record.snapshot(now)))
            .collect()
    }
}

impl History {
    pub fn start(&self, key: String) -> Session {
        let mut records = self.0.lock().unwrap();
        let record = records.entry(key.clone()).or_default();
        *record = Record {
            total_ms: record.total_ms,
            all_sessions: Some(record.average()),
            without_exact_60: record.without_exact_60,
            started: Some(Instant::now()),
            ..Record::default()
        };
        Session {
            history: self.clone(),
            key,
        }
    }

    #[cfg(test)]
    pub fn averages(&self, include_exact_60: bool) -> BTreeMap<String, f64> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(key, record)| {
                record
                    .selected_average(include_exact_60)
                    .map(|fps| (key.clone(), fps))
            })
            .collect()
    }

    pub fn reset_fps(&self, key: &str) {
        if let Some(record) = self.0.lock().unwrap().get_mut(key) {
            record.all_sessions = Some(FpsAverage::default());
            record.without_exact_60 = FpsAverage::default();
        }
    }

    pub fn profile(&self, key: &str, include_exact_60: bool) -> Option<ProfileStats> {
        let records = self.0.lock().unwrap();
        let record = records.get(key)?;
        Some(ProfileStats {
            playtime: duration(record.snapshot(Instant::now()).total_ms),
            average_fps: record.selected_average(include_exact_60),
            #[cfg(test)]
            latest_fps: record.latest_fps,
            #[cfg(test)]
            playing: record.started.is_some(),
        })
    }
}

fn duration(ms: u64) -> String {
    let seconds = ms / 1000;
    match seconds {
        0 => "<1s".into(),
        1..60 => format!("{seconds}s"),
        60..3600 => format!("{}m", seconds / 60),
        _ => format!("{}h {}m", seconds / 3600, seconds / 60 % 60),
    }
}

pub struct Session {
    history: History,
    key: String,
}

impl Session {
    pub fn observe(&self, line: &str) {
        if let Some(value) = renderer_fps(line) {
            let mut records = self.history.0.lock().unwrap();
            let record = records.get_mut(&self.key).unwrap();
            let mut avg = record.average();
            avg.observe(value);
            record.all_sessions = Some(avg);
            if value != 60.0 {
                record.without_exact_60.observe(value);
            }
            record.samples += 1;
            record.mean_fps += (value - record.mean_fps) / record.samples as f64;
            record.latest_fps = Some(value);
        }
    }

    pub fn stop_clock(&self) {
        let mut records = self.history.0.lock().unwrap();
        let record = records.get_mut(&self.key).unwrap();
        *record = record.snapshot(Instant::now());
    }

    pub fn summary(&self) -> String {
        let records = self.history.0.lock().unwrap();
        let record = &records[&self.key];
        let fps = if record.samples == 0 {
            "FPS unavailable (no renderer readings)".into()
        } else {
            format!(
                "{:.1} FPS average across {} renderer readings",
                record.mean_fps, record.samples
            )
        };
        format!("Session playtime {} · {fps}", duration(record.session_ms))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop_clock();
    }
}

// Both plain and ANSI-colored tracing prefixes end in whitespace before the FPS value.
fn renderer_fps(line: &str) -> Option<f64> {
    let (prefix, rest) = line.split_once(" fps, ")?;
    if !prefix.contains("victory_gpu::host") {
        return None;
    }
    let (ms, _) = rest.split_once(" ms/frame")?;
    let ms: f64 = ms.parse().ok()?;
    let fps: f64 = prefix.split_whitespace().next_back()?.parse().ok()?;
    (ms.is_finite() && ms >= 0.0 && fps.is_finite() && fps >= 0.0).then_some(fps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn checkpoints_include_live_time_once_and_totals_survive_relaunch() {
        let history = History::default();
        let session = history.start("scarlett".into());
        history
            .0
            .lock()
            .unwrap()
            .get_mut("scarlett")
            .unwrap()
            .started = Some(Instant::now() - Duration::from_secs(90));
        session.observe("\x1b[2mvictory_gpu::host\x1b[0m\x1b[2m:\x1b[0m 12.4 fps, 80.5 ms/frame (42.2 preparing)");
        let live = history.profile("scarlett", true).unwrap();
        assert!(live.playing);
        assert_eq!(live.playtime, "1m");
        assert_eq!(live.latest_fps, Some(12.4));
        let checkpoint = serde_json::to_string(&history).unwrap();
        let restored: History = serde_json::from_str(&checkpoint).unwrap();
        assert!(!restored.profile("scarlett", true).unwrap().playing);
        assert!(restored.profile("unplayed", true).is_none());
        session.stop_clock();
        let ms = history.0.lock().unwrap()["scarlett"].total_ms;
        drop(session);
        assert_eq!(history.0.lock().unwrap()["scarlett"].total_ms, ms);
        let next = restored.start("scarlett".into());
        assert_eq!(restored.profile("scarlett", true).unwrap().latest_fps, None);
        drop(next);
        assert_eq!(restored.profile("scarlett", true).unwrap().playtime, "1m");
        assert_eq!(
            restored.profile("scarlett", true).unwrap().average_fps,
            Some(12.4)
        );
    }

    #[test]
    fn older_readings_and_new_sessions_are_sample_weighted_and_reset_independently() {
        let history: History = serde_json::from_str(r#"{"a":{"total_ms":120000,"session_ms":10000,"samples":3,"mean_fps":20.0,"latest_fps":22.0}}"#).unwrap();
        assert_eq!(history.averages(true)["a"], 20.0);
        let b = history.start("b".into());
        b.observe("victory_gpu::host: 50 fps, 20 ms/frame");
        let a = history.start("a".into());
        a.observe("victory_gpu::host: 60 fps, 16.7 ms/frame");
        assert_eq!(history.averages(true)["a"], 30.0);
        history.reset_fps("a");
        let reloaded: History =
            serde_json::from_str(&serde_json::to_string(&history).unwrap()).unwrap();
        assert!(!reloaded.averages(true).contains_key("a"));
        assert_eq!(reloaded.averages(true)["b"], 50.0);
        assert_eq!(reloaded.profile("a", true).unwrap().playtime, "2m");
        a.observe("victory_gpu::host: 40 fps, 25 ms/frame");
        assert_eq!(history.averages(true)["a"], 40.0);
        assert!(a
            .summary()
            .contains("50.0 FPS average across 2 renderer readings"));
        drop(a);
        drop(history.start("a".into()));
        assert_eq!(history.averages(true)["a"], 40.0);
    }

    #[test]
    fn exact_filter_is_reversible_and_never_guesses_from_legacy_means() {
        let old: History = serde_json::from_str(r#"{"p":{"total_ms":1000,"session_ms":1000,"samples":2,"mean_fps":45.0,"latest_fps":60.0}}"#).unwrap();
        assert_eq!(old.averages(true)["p"], 45.0);
        assert!(old.averages(false).is_empty());
        let session = old.start("p".into());
        session.observe("victory_gpu::host: 60.0 fps, 16.7 ms/frame");
        assert!(old.averages(false).is_empty());
        for sample in [59.9, 60.1, 30.0] {
            session.observe(&format!("victory_gpu::host: {sample} fps, 16.7 ms/frame"));
        }
        assert_eq!(old.profile("p", false).unwrap().average_fps, Some(50.0));
        assert_eq!(old.averages(true)["p"], 50.0);
        drop(session);
        let next = old.start("p".into());
        next.observe("victory_gpu::host: 60 fps, 16.7 ms/frame");
        let restored: History =
            serde_json::from_str(&serde_json::to_string(&old).unwrap()).unwrap();
        assert_eq!(restored.averages(false)["p"], 50.0);
        assert!(restored.averages(true)["p"] > 51.0);
        restored.reset_fps("p");
        assert!(restored.averages(false).is_empty() && restored.averages(true).is_empty());
    }

    #[test]
    fn only_finite_renderer_measurements_count() {
        assert_eq!(renderer_fps("target 60 fps, 16.7 ms/frame"), None);
        assert_eq!(renderer_fps("victory_gpu::host: NaN fps, 0 ms/frame"), None);
        assert_eq!(
            renderer_fps("victory_gpu::host: 20 fps, unknown ms/frame"),
            None
        );
        assert_eq!(
            renderer_fps("victory_gpu::host: 0.0 fps, 0.0 ms/frame"),
            Some(0.0)
        );
    }
}
