//! The player's PC, looked at once in the background when the launcher
//! starts: the processor, the graphics cards (with their video memory) and
//! the system memory, so the window can say whether a build is likely to run
//! here and which builds to recommend.
//!
//! On Windows one hidden PowerShell call answers everything; elsewhere
//! (development only) the memory is read and the graphics list stays short.

use serde::Serialize;

/// What the window gets (`SystemInfo` in src/lib/types.ts).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct SystemInfo {
    pub gpus: Vec<Gpu>,
    pub ram_gb: Option<f64>,
    /// The processor's name, as Windows writes it ("AMD Ryzen 7 5800X 8-Core Processor").
    pub cpu: Option<String>,
    /// Logical processors (threads), summed over every socket.
    pub cpu_cores: Option<u32>,
    /// Running under Rosetta 2 on Apple silicon (a translated process).
    #[serde(default)]
    pub translated: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Gpu {
    pub name: String,
    pub vendor: Vendor,
    pub vram_gb: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(crate) enum Vendor {
    #[serde(rename = "NVIDIA")]
    Nvidia,
    #[serde(rename = "AMD")]
    Amd,
    Intel,
    Apple,
    Other,
}

/// The maker, from a graphics card's name.
pub(crate) fn vendor(name: &str) -> Vendor {
    let lower = name.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let has = |list: &[&str]| words.iter().any(|w| list.contains(w));
    if has(&[
        "nvidia", "geforce", "rtx", "gtx", "quadro", "tesla", "titan",
    ]) {
        Vendor::Nvidia
    } else if has(&["amd", "radeon", "ati"]) {
        Vendor::Amd
    } else if has(&["intel", "arc", "iris", "uhd", "hd"]) {
        Vendor::Intel
    } else if has(&["apple"]) {
        Vendor::Apple
    } else {
        Vendor::Other
    }
}

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// System memory in whole gigabytes.
pub(crate) fn ram_gb(bytes: f64) -> Option<f64> {
    (bytes.is_finite() && bytes > 0.0).then(|| (bytes / GIB).round().max(1.0))
}

/// Video memory: whole gigabytes when it is close to one (a 24 GB card says
/// 23.99), else to one decimal (small shared amounts).
pub(crate) fn vram_gb(bytes: f64) -> Option<f64> {
    if !bytes.is_finite() || bytes <= 0.0 {
        return None;
    }
    let gb = bytes / GIB;
    let whole = gb.round();
    if whole >= 1.0 && (gb - whole).abs() < 0.1 {
        Some(whole)
    } else {
        Some(((gb * 10.0).round() / 10.0).max(0.1))
    }
}

/// Adapters that are not graphics cards: remote desktop, streaming and
/// virtual displays, and Windows' stand-in when no driver is installed.
fn not_a_card(name: &str) -> bool {
    let l = name.to_lowercase();
    [
        "virtual",
        "indirect display",
        "remote display",
        "basic display",
        "basic render",
        "hyper-v",
        "mirror driver",
        "paravirtual",
    ]
    .iter()
    .any(|w| l.contains(w))
}

/// The cards, matched with the video memory the drivers wrote down (by name).
fn cards(names: &[String], adapters: &[(String, f64)]) -> Vec<Gpu> {
    let key = |s: &str| s.trim().to_lowercase();
    let mut gpus: Vec<Gpu> = names
        .iter()
        .map(|n| n.trim())
        .filter(|n| !n.is_empty() && !not_a_card(n))
        .map(|name| {
            let bytes = adapters
                .iter()
                .find(|(desc, _)| key(desc) == key(name))
                .map(|a| a.1);
            Gpu {
                name: name.to_owned(),
                vendor: vendor(name),
                vram_gb: bytes.and_then(vram_gb),
            }
        })
        .collect();
    // The biggest card first: that is the one games use.
    gpus.sort_by(|a, b| {
        b.vram_gb
            .unwrap_or(0.0)
            .total_cmp(&a.vram_gb.unwrap_or(0.0))
    });
    gpus
}

/// Looks at this PC (can take a few seconds; never panics, gives up after ~8 s).
pub(crate) fn detect() -> SystemInfo {
    std::panic::catch_unwind(detect_inner).unwrap_or_default()
}

#[cfg(windows)]
fn detect_inner() -> SystemInfo {
    windows::detect()
}

#[cfg(target_os = "macos")]
fn detect_inner() -> SystemInfo {
    macos::detect()
}

/// The share of unified memory a game can sensibly use: Apple's own
/// `recommendedMaxWorkingSetSize` is about three quarters, and the rest of
/// the system needs the remainder. Kept shy of it so a Fit warning is not
/// given to a build that fits.
const UNIFIED_SHARE: f64 = 0.7;

/// The graphics memory budget to report for a chip with unified memory.
fn unified_budget(ram_gb: Option<f64>) -> Option<f64> {
    let ram = ram_gb?;
    // Half-gigabyte steps, never less than one.
    Some(((ram * UNIFIED_SHARE * 2.0).round() / 2.0).max(1.0))
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn detect_inner() -> SystemInfo {
    let ram_gb = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| meminfo_bytes(&text))
        .and_then(ram_gb);
    let (cpu, cpu_cores) = std::fs::read_to_string("/proc/cpuinfo")
        .map(|text| parse_cpuinfo(&text))
        .unwrap_or((None, None));
    SystemInfo {
        gpus: linux_gpus(std::path::Path::new("/sys/bus/pci/devices")),
        ram_gb,
        cpu,
        cpu_cores,
        translated: false,
    }
}

/// The processor's name and its logical processor count, from /proc/cpuinfo.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn parse_cpuinfo(text: &str) -> (Option<String>, Option<u32>) {
    let name = text
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, value)| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let threads = text.lines().filter(|l| l.starts_with("processor")).count();
    (name, u32::try_from(threads).ok().filter(|n| *n > 0))
}

/// The graphics cards on the PCI bus (PCI class 03: display controllers), from sysfs.
/// The name is the maker and the device id; video memory is read where the driver
/// reports it (AMD's amdgpu does, NVIDIA's driver does not).
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn linux_gpus(bus: &std::path::Path) -> Vec<Gpu> {
    let read = |path: std::path::PathBuf| std::fs::read_to_string(path).ok();
    let mut gpus: Vec<Gpu> = std::fs::read_dir(bus)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let dir = entry.path();
            let class = read(dir.join("class"))?;
            if !class.trim().starts_with("0x03") {
                return None;
            }
            let maker = match read(dir.join("vendor"))?.trim() {
                "0x10de" => "NVIDIA",
                "0x1002" => "AMD",
                "0x8086" => "Intel",
                _ => "Other",
            };
            let device = read(dir.join("device")).unwrap_or_default();
            let name = format!("{maker} graphics card {}", device.trim());
            let vram_gb = read(dir.join("mem_info_vram_total"))
                .and_then(|bytes| bytes.trim().parse::<f64>().ok())
                .and_then(vram_gb);
            Some(Gpu {
                vendor: vendor(&name),
                name,
                vram_gb,
            })
        })
        .collect();
    // The biggest card first: that is the one games use.
    gpus.sort_by(|a, b| b.vram_gb.unwrap_or(0.0).total_cmp(&a.vram_gb.unwrap_or(0.0)));
    gpus
}

/// `MemTotal` from /proc/meminfo, in bytes.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn meminfo_bytes(text: &str) -> Option<f64> {
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kb: f64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024.0)
}

/// What the PowerShell script prints, as JSON.
#[cfg_attr(not(windows), allow(dead_code))]
fn parse_report(json: &str) -> Option<SystemInfo> {
    let v: serde_json::Value =
        serde_json::from_str(json.trim().trim_start_matches('\u{feff}')).ok()?;
    // PowerShell writes a list of one as the item itself, and none as null.
    let list = |v: &serde_json::Value| -> Vec<serde_json::Value> {
        match v {
            serde_json::Value::Array(items) => items.clone(),
            serde_json::Value::Null => Vec::new(),
            other => vec![other.clone()],
        }
    };
    let number = |v: &serde_json::Value| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
    };
    let names: Vec<String> = list(&v["gpus"])
        .iter()
        .filter_map(|n| n.as_str().map(str::to_owned))
        .collect();
    let adapters: Vec<(String, f64)> = list(&v["adapters"])
        .iter()
        .filter_map(|a| Some((a["desc"].as_str()?.to_owned(), number(&a["bytes"])?)))
        .collect();
    // The memory modules' total, else what Windows can use (a little less).
    let ram = number(&v["installed"])
        .filter(|b| *b > 0.0)
        .or_else(|| number(&v["ram"]));
    let cpu = v["cpu"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let cpu_cores = number(&v["cpu_cores"])
        .filter(|n| *n >= 1.0)
        .map(|n| n.round() as u32);
    Some(SystemInfo {
        gpus: cards(&names, &adapters),
        ram_gb: ram.and_then(ram_gb),
        cpu,
        cpu_cores,
        translated: false,
    })
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    /// The graphics cards, the video memory each driver wrote in the registry
    /// (`AdapterRAM` stops at 4 GB, `qwMemorySize` does not) and the memory.
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
function Bytes($v) {
  if ($v -is [byte[]]) {
    if ($v.Length -ge 8) { return [double][BitConverter]::ToUInt64($v, 0) }
    if ($v.Length -ge 4) { return [double][BitConverter]::ToUInt32($v, 0) }
    return $null
  }
  if ($null -ne $v) { return [double]$v }
  return $null
}
$adapters = @()
foreach ($key in @(Get-ItemProperty 'HKLM:\SYSTEM\ControlSet001\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0*')) {
  $b = Bytes $key.'HardwareInformation.qwMemorySize'
  if ($null -eq $b) { $b = Bytes $key.'HardwareInformation.MemorySize' }
  if ($key.DriverDesc -and $b) { $adapters += [pscustomobject]@{ desc = [string]$key.DriverDesc; bytes = $b } }
}
$gpus = @(Get-CimInstance Win32_VideoController | ForEach-Object { [string]$_.Name })
$ram = [double](Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
$installed = [double]((Get-CimInstance Win32_PhysicalMemory | Measure-Object -Property Capacity -Sum).Sum)
$cpuName = [string](Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)
$cores = [int]((Get-CimInstance Win32_Processor | Measure-Object -Property NumberOfLogicalProcessors -Sum).Sum)
[pscustomobject]@{ gpus = $gpus; adapters = $adapters; ram = $ram; installed = $installed; cpu = $cpuName; cpu_cores = $cores } | ConvertTo-Json -Compress -Depth 4
"#;

    const TIMEOUT: Duration = Duration::from_secs(8);

    pub(super) fn detect() -> SystemInfo {
        run().and_then(|out| parse_report(&out)).unwrap_or_default()
    }

    fn powershell() -> std::path::PathBuf {
        let root = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "C:\\Windows".into());
        let full = root
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        if full.is_file() {
            full
        } else {
            "powershell.exe".into()
        }
    }

    fn run() -> Option<String> {
        let utf16: Vec<u8> = SCRIPT.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let encoded = base64(&utf16);
        let mut c = Command::new(powershell());
        c.args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            encoded.as_str(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
        crate::hide_window(&mut c);
        let mut child = c.spawn().ok()?;
        let mut stdout = child.stdout.take()?;
        let reader = std::thread::spawn(move || {
            let mut out = String::new();
            let _ = stdout.read_to_string(&mut out);
            out
        });
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if start.elapsed() < TIMEOUT => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        }
        reader.join().ok()
    }

    fn base64(bytes: &[u8]) -> String {
        const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let n = (chunk[0] as u32) << 16
                | (*chunk.get(1).unwrap_or(&0) as u32) << 8
                | *chunk.get(2).unwrap_or(&0) as u32;
            for i in 0..4 {
                if i <= chunk.len() {
                    out.push(ABC[(n >> (18 - 6 * i) & 63) as usize] as char);
                } else {
                    out.push('=');
                }
            }
        }
        out
    }

    #[test]
    fn base64_matches_the_standard_alphabet() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    /// One `sysctl` value, trimmed; `None` when the key is unknown.
    fn sysctl(key: &str) -> Option<String> {
        let out = Command::new("/usr/sbin/sysctl")
            .args(["-n", key])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
            .filter(|value| !value.is_empty())
    }

    /// A "4 GB" / "1536 MB" memory figure in bytes.
    fn bytes_from_text(text: &str) -> Option<f64> {
        let number: f64 = text
            .split_whitespace()
            .find(|word| word.chars().next().is_some_and(|c| c.is_ascii_digit()))?
            .trim_end_matches(|c: char| !c.is_ascii_digit())
            .parse()
            .ok()?;
        let unit = text.to_lowercase();
        let factor = if unit.contains("gb") {
            GIB
        } else if unit.contains("mb") {
            GIB / 1024.0
        } else {
            return None;
        };
        Some(number * factor)
    }

    /// The displays `system_profiler` reports: (name, video memory when given).
    fn parse_displays(json: &str) -> Vec<(String, Option<f64>)> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
            return Vec::new();
        };
        let list = value["SPDisplaysDataType"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        list.iter()
            .filter_map(|entry| {
                let name = entry["sppci_model"]
                    .as_str()
                    .or_else(|| entry["_name"].as_str())?
                    .trim()
                    .to_owned();
                if name.is_empty() {
                    return None;
                }
                let memory = [
                    "spdisplays_vram",
                    "spdisplays_vram_shared",
                    "_spdisplays_vram",
                ]
                .iter()
                .find_map(|key| entry[*key].as_str().and_then(bytes_from_text));
                Some((name, memory))
            })
            .collect()
    }

    /// The graphics cards `system_profiler` lists (a couple of seconds; gives
    /// up after three so a wedged profiler never holds the start).
    fn displays() -> Vec<(String, Option<f64>)> {
        let mut c = Command::new("/usr/sbin/system_profiler");
        c.args(["SPDisplaysDataType", "-json"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let Ok(mut child) = c.spawn() else {
            return Vec::new();
        };
        let mut stdout = child.stdout.take().expect("piped");
        let reader = std::thread::spawn(move || {
            let mut out = String::new();
            let _ = stdout.read_to_string(&mut out);
            out
        });
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if start.elapsed() < Duration::from_secs(3) => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Vec::new();
                }
            }
        }
        reader
            .join()
            .ok()
            .map(|out| parse_displays(&out))
            .unwrap_or_default()
    }

    pub(super) fn detect() -> SystemInfo {
        let cpu = sysctl("machdep.cpu.brand_string");
        let ram = sysctl("hw.memsize")
            .and_then(|value| value.parse::<f64>().ok())
            .and_then(ram_gb);
        let translated = sysctl("sysctl.proc_translated").as_deref() == Some("1");
        let cpu_cores = sysctl("hw.logicalcpu")
            .or_else(|| sysctl("hw.ncpu"))
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|n| *n >= 1);
        // A whole-chip Apple GPU shares the system's memory; a discrete or
        // Intel Mac card has its own and is read from `system_profiler`.
        let gpus = match cpu.clone().filter(|chip| chip.starts_with("Apple")) {
            Some(chip) => vec![Gpu {
                name: format!("{chip} (Unified Memory)"),
                vendor: Vendor::Apple,
                vram_gb: unified_budget(ram),
            }],
            None => {
                let found: Vec<Gpu> = displays()
                    .into_iter()
                    .filter(|(name, _)| !not_a_card(name))
                    .map(|(name, bytes)| Gpu {
                        vendor: vendor(&name),
                        name,
                        vram_gb: bytes.and_then(vram_gb),
                    })
                    .collect();
                // No answer (killed, or a profiler that failed): name the
                // machine's chip so the fit check still has something to go on.
                if found.is_empty() {
                    chip_gpu(&sysctl("hw.model")).into_iter().collect()
                } else {
                    found
                }
            }
        };
        SystemInfo {
            gpus,
            ram_gb: ram,
            cpu,
            cpu_cores,
            translated,
        }
    }

    /// The card a fallback can name when `system_profiler` gave nothing: an
    /// Intel Mac's integrated-graphics family from the model identifier.
    fn chip_gpu(model: &Option<String>) -> Option<Gpu> {
        let model = model.as_ref()?;
        let name = if model.starts_with("MacBookPro1") || model.starts_with("MacBookAir1") {
            "Intel integrated graphics"
        } else if model.starts_with("MacPro") || model.starts_with("iMac") {
            "Graphics"
        } else {
            "Unknown graphics"
        };
        Some(Gpu {
            name: name.to_owned(),
            vendor: Vendor::Other,
            vram_gb: None,
        })
    }

    #[test]
    fn displays_are_read_from_the_profiler_report() {
        let report = r#"{"SPDisplaysDataType":[{"sppci_model":"Apple M3 Pro","spdisplays_vram_shared":"18 GB"},
            {"_name":"Apple Paravirtual device","spdisplays_vram":"0 MB"},
            {"sppci_model":"AMD Radeon Pro 5500M","spdisplays_vram":"4 GB"}]}"#;
        let found = parse_displays(report);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].0, "Apple M3 Pro");
        assert_eq!(found[0].1, Some(18.0 * GIB));
        assert_eq!(found[2].1, Some(4.0 * GIB));
        assert!(parse_displays("not json").is_empty());
        assert!(parse_displays("{}").is_empty());
    }

    #[test]
    fn a_missing_profiler_answer_still_names_the_graphics() {
        let Some(gpu) = chip_gpu(&Some("MacBookPro16,1".to_string())) else {
            panic!("a model identifier names a card")
        };
        assert_eq!(gpu.name, "Intel integrated graphics");
        assert!(chip_gpu(&None).is_none());
    }

    #[test]
    fn the_unified_budget_is_a_share_of_memory_in_half_gigabyte_steps() {
        assert_eq!(unified_budget(Some(16.0)), Some(11.0)); // 11.2, shy of Apple's 75%
        assert_eq!(unified_budget(Some(8.0)), Some(5.5));
        assert_eq!(unified_budget(Some(64.0)), Some(45.0));
        assert_eq!(unified_budget(None), None);
        assert!(unified_budget(Some(1.0)).unwrap() >= 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendors_come_from_the_card_name() {
        for (name, want) in [
            ("NVIDIA GeForce RTX 4090", Vendor::Nvidia),
            ("NVIDIA RTX A4000", Vendor::Nvidia),
            ("GeForce GTX 1080 Ti", Vendor::Nvidia),
            ("Quadro P2000", Vendor::Nvidia),
            ("AMD Radeon RX 7900 XTX", Vendor::Amd),
            ("Radeon(TM) Graphics", Vendor::Amd),
            ("AMD Radeon(TM) 780M", Vendor::Amd),
            ("Intel(R) Arc(TM) A770 Graphics", Vendor::Intel),
            ("Intel(R) UHD Graphics 630", Vendor::Intel),
            ("Intel(R) Iris(R) Xe Graphics", Vendor::Intel),
            ("Apple M2 Pro", Vendor::Apple),
            ("Moore Threads MTT S80", Vendor::Other),
            ("Research Graphics", Vendor::Other),
        ] {
            assert_eq!(vendor(name), want, "{name}");
        }
    }

    #[test]
    fn gigabytes_are_rounded_for_people() {
        assert_eq!(ram_gb(34_225_520_640.0), Some(32.0)); // 31.87 GiB usable of 32
        assert_eq!(ram_gb(17_179_869_184.0), Some(16.0));
        assert_eq!(ram_gb(0.0), None);
        assert_eq!(ram_gb(f64::NAN), None);
        assert_eq!(vram_gb(25_757_220_864.0), Some(24.0)); // RTX 4090
        assert_eq!(vram_gb(12_884_901_888.0), Some(12.0));
        assert_eq!(vram_gb(8_573_157_376.0), Some(8.0)); // 7.98 GiB
        assert_eq!(vram_gb(536_870_912.0), Some(0.5));
        assert_eq!(vram_gb(134_217_728.0), Some(0.1));
        assert_eq!(vram_gb(-1.0), None);
    }

    #[test]
    fn the_powershell_report_is_read_with_its_quirks() {
        let report = r#"{"gpus":["NVIDIA GeForce RTX 4070","Intel(R) UHD Graphics 770","Parsec Virtual Display Adapter"],
            "adapters":[{"desc":"Intel(R) UHD Graphics 770","bytes":134217728},{"desc":"NVIDIA GeForce RTX 4070","bytes":12884901888}],
            "ram":34225520640,"installed":34359738368,
            "cpu":"  AMD Ryzen 7 5800X 8-Core Processor  ","cpu_cores":16}"#;
        let info = parse_report(report).unwrap();
        assert_eq!(info.ram_gb, Some(32.0));
        assert_eq!(
            info.cpu.as_deref(),
            Some("AMD Ryzen 7 5800X 8-Core Processor")
        );
        assert_eq!(info.cpu_cores, Some(16));
        assert_eq!(info.gpus.len(), 2);
        assert_eq!(
            info.gpus[0],
            Gpu {
                name: "NVIDIA GeForce RTX 4070".into(),
                vendor: Vendor::Nvidia,
                vram_gb: Some(12.0)
            }
        );
        assert_eq!(info.gpus[1].vendor, Vendor::Intel);
        // One card comes as the item itself; no registry match leaves the memory unknown.
        let one = parse_report("\u{feff}{\"gpus\":\"AMD Radeon RX 6600\",\"adapters\":null,\"ram\":17179869184,\"installed\":0,\"cpu\":\"\",\"cpu_cores\":0}").unwrap();
        assert_eq!(
            one.gpus,
            vec![Gpu {
                name: "AMD Radeon RX 6600".into(),
                vendor: Vendor::Amd,
                vram_gb: None
            }]
        );
        assert_eq!(one.ram_gb, Some(16.0));
        // An empty processor name and a zero core count are no answer at all.
        assert_eq!((one.cpu, one.cpu_cores), (None, None));
        assert!(parse_report("not json").is_none());
    }

    #[test]
    fn linux_processor_and_threads_are_read_from_cpuinfo() {
        let text = "processor\t: 0\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD Ryzen 7 5800X 8-Core Processor\n\nprocessor\t: 1\nmodel name\t: AMD Ryzen 7 5800X 8-Core Processor\n";
        let (name, threads) = parse_cpuinfo(text);
        assert_eq!(name.as_deref(), Some("AMD Ryzen 7 5800X 8-Core Processor"));
        assert_eq!(threads, Some(2));
        assert_eq!(parse_cpuinfo(""), (None, None));
    }

    #[test]
    fn linux_graphics_cards_come_from_the_pci_display_class() {
        let root = std::env::temp_dir().join(format!("vl-pci-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let card = |name: &str, class: &str, vendor: &str, vram: Option<&str>| {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("class"), class).unwrap();
            std::fs::write(dir.join("vendor"), vendor).unwrap();
            std::fs::write(dir.join("device"), "0x2504\n").unwrap();
            if let Some(vram) = vram {
                std::fs::write(dir.join("mem_info_vram_total"), vram).unwrap();
            }
        };
        card("0000-01-00.0", "0x030000\n", "0x10de\n", None);
        card("0000-06-00.0", "0x030000\n", "0x1002\n", Some("17179869184\n"));
        card("0000-00-1f.3", "0x040300\n", "0x8086\n", None);
        let gpus = linux_gpus(&root);
        assert_eq!(gpus.len(), 2, "the audio device is not a card");
        assert_eq!(gpus[0].vendor, Vendor::Amd);
        assert_eq!(gpus[0].vram_gb, Some(16.0));
        assert_eq!(gpus[1].vendor, Vendor::Nvidia);
        assert_eq!(gpus[1].vram_gb, None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn meminfo_total_is_read() {
        assert_eq!(
            meminfo_bytes("MemTotal:       16303428 kB\nMemFree: 1 kB\n"),
            Some(16_694_710_272.0)
        );
        assert_eq!(meminfo_bytes("nothing"), None);
    }

    #[test]
    fn serializes_as_the_window_expects() {
        let info = SystemInfo {
            gpus: vec![Gpu {
                name: "X".into(),
                vendor: Vendor::Nvidia,
                vram_gb: None,
            }],
            ram_gb: Some(16.0),
            cpu: Some("Y".into()),
            cpu_cores: Some(8),
            translated: false,
        };
        assert_eq!(
            serde_json::to_string(&info).unwrap(),
            r#"{"gpus":[{"name":"X","vendor":"NVIDIA","vram_gb":null}],"ram_gb":16.0,"cpu":"Y","cpu_cores":8,"translated":false}"#
        );
    }
}
