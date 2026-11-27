//! The player's PC, looked at once in the background when the launcher
//! starts: the graphics cards (with their video memory) and the system
//! memory, so the window can say whether a project is likely to run here.
//!
//! On Windows one hidden PowerShell call answers everything; elsewhere
//! (development only) the memory is read and the graphics list stays short.

use serde::Serialize;

/// What the window gets (`SystemInfo` in src/lib/types.ts).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct SystemInfo {
    pub gpus: Vec<Gpu>,
    pub ram_gb: Option<f64>,
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
    let words: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
    let has = |list: &[&str]| words.iter().any(|w| list.contains(w));
    if has(&["nvidia", "geforce", "rtx", "gtx", "quadro", "tesla", "titan"]) {
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
    ["virtual", "indirect display", "remote display", "basic display", "basic render", "hyper-v", "mirror driver"]
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
            let bytes = adapters.iter().find(|(desc, _)| key(desc) == key(name)).map(|a| a.1);
            Gpu { name: name.to_owned(), vendor: vendor(name), vram_gb: bytes.and_then(vram_gb) }
        })
        .collect();
    // The biggest card first: that is the one games use.
    gpus.sort_by(|a, b| b.vram_gb.unwrap_or(0.0).total_cmp(&a.vram_gb.unwrap_or(0.0)));
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
    let sysctl = |key: &str| -> Option<String> {
        let out = std::process::Command::new("/usr/sbin/sysctl").args(["-n", key]).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    };
    let ram_gb = sysctl("hw.memsize").and_then(|s| s.parse::<f64>().ok()).and_then(ram_gb);
    // Apple silicon: the graphics are part of the chip and share its memory.
    let gpus = sysctl("machdep.cpu.brand_string")
        .filter(|chip| chip.starts_with("Apple"))
        .map(|chip| vec![Gpu { vendor: vendor(&chip), name: chip, vram_gb: None }])
        .unwrap_or_default();
    SystemInfo { gpus, ram_gb }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn detect_inner() -> SystemInfo {
    let ram_gb = std::fs::read_to_string("/proc/meminfo").ok().and_then(|text| meminfo_bytes(&text)).and_then(ram_gb);
    SystemInfo { gpus: Vec::new(), ram_gb }
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
    let v: serde_json::Value = serde_json::from_str(json.trim().trim_start_matches('\u{feff}')).ok()?;
    // PowerShell writes a list of one as the item itself, and none as null.
    let list = |v: &serde_json::Value| -> Vec<serde_json::Value> {
        match v {
            serde_json::Value::Array(items) => items.clone(),
            serde_json::Value::Null => Vec::new(),
            other => vec![other.clone()],
        }
    };
    let number = |v: &serde_json::Value| v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()));
    let names: Vec<String> = list(&v["gpus"]).iter().filter_map(|n| n.as_str().map(str::to_owned)).collect();
    let adapters: Vec<(String, f64)> = list(&v["adapters"])
        .iter()
        .filter_map(|a| Some((a["desc"].as_str()?.to_owned(), number(&a["bytes"])?)))
        .collect();
    // The memory modules' total, else what Windows can use (a little less).
    let ram = number(&v["installed"]).filter(|b| *b > 0.0).or_else(|| number(&v["ram"]));
    Some(SystemInfo { gpus: cards(&names, &adapters), ram_gb: ram.and_then(ram_gb) })
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
[pscustomobject]@{ gpus = $gpus; adapters = $adapters; ram = $ram; installed = $installed } | ConvertTo-Json -Compress -Depth 4
"#;

    const TIMEOUT: Duration = Duration::from_secs(8);

    pub(super) fn detect() -> SystemInfo {
        run().and_then(|out| parse_report(&out)).unwrap_or_default()
    }

    fn powershell() -> std::path::PathBuf {
        let root = std::env::var_os("SystemRoot").map(std::path::PathBuf::from).unwrap_or_else(|| "C:\\Windows".into());
        let full = root.join("System32").join("WindowsPowerShell").join("v1.0").join("powershell.exe");
        if full.is_file() { full } else { "powershell.exe".into() }
    }

    fn run() -> Option<String> {
        let utf16: Vec<u8> = SCRIPT.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let encoded = base64(&utf16);
        let mut c = Command::new(powershell());
        c.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", encoded.as_str()])
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
                Ok(None) if start.elapsed() < TIMEOUT => std::thread::sleep(Duration::from_millis(50)),
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
            let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
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
            "ram":34225520640,"installed":34359738368}"#;
        let info = parse_report(report).unwrap();
        assert_eq!(info.ram_gb, Some(32.0));
        assert_eq!(info.gpus.len(), 2);
        assert_eq!(info.gpus[0], Gpu { name: "NVIDIA GeForce RTX 4070".into(), vendor: Vendor::Nvidia, vram_gb: Some(12.0) });
        assert_eq!(info.gpus[1].vendor, Vendor::Intel);
        // One card comes as the item itself; no registry match leaves the memory unknown.
        let one = parse_report("\u{feff}{\"gpus\":\"AMD Radeon RX 6600\",\"adapters\":null,\"ram\":17179869184,\"installed\":0}").unwrap();
        assert_eq!(one.gpus, vec![Gpu { name: "AMD Radeon RX 6600".into(), vendor: Vendor::Amd, vram_gb: None }]);
        assert_eq!(one.ram_gb, Some(16.0));
        assert!(parse_report("not json").is_none());
    }

    #[test]
    fn meminfo_total_is_read() {
        assert_eq!(meminfo_bytes("MemTotal:       16303428 kB\nMemFree: 1 kB\n"), Some(16_694_710_272.0));
        assert_eq!(meminfo_bytes("nothing"), None);
    }

    #[test]
    fn serializes_as_the_window_expects() {
        let info = SystemInfo { gpus: vec![Gpu { name: "X".into(), vendor: Vendor::Nvidia, vram_gb: None }], ram_gb: Some(16.0) };
        assert_eq!(serde_json::to_string(&info).unwrap(), r#"{"gpus":[{"name":"X","vendor":"NVIDIA","vram_gb":null}],"ram_gb":16.0}"#);
    }
}
