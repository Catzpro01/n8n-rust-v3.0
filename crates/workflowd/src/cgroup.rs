// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// How often the governor re-reads cgroup counters. The kernel files are cheap
/// key/value text and 250 ms is coarse enough to avoid syscall hot-spots while
/// keeping throttling and pressure visible within one checkpoint interval.
pub const SAMPLE_INTERVAL: Duration = Duration::from_millis(250);

/// How often disk stats are refreshed, since they require running `df -Pk`.
pub const DISK_SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

/// The default managed-disk reserve that must remain available on the state
/// partition. The daemon treats crossing this boundary as storage pressure.
pub const DEFAULT_MANAGED_DISK_RESERVE_BYTES: u64 = 10 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct ResourceIdentity {
    pub cgroup_version: u8,
    pub cgroup_path: String,
    pub cpu: ControllerIdentity,
    pub memory: ControllerIdentity,
    pub io: ControllerIdentity,
    pub pids: ControllerIdentity,
    pub pressure: ControllerIdentity,
    pub disk: DiskIdentity,
    pub accelerator: AcceleratorIdentity,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControllerIdentity {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub values: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskIdentity {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub state_directory: String,
    pub values: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AcceleratorIdentity {
    pub policy: &'static str,
    pub eligible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ResourceSample {
    pub sampled_at_millis: i64,
    pub cpu: CpuSample,
    pub memory: MemorySample,
    pub io: IoSample,
    pub pids: PidsSample,
    pub pressure: PressureSample,
    pub disk: DiskSample,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CpuSample {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_cores: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_usec: Option<u64>,
    pub usage_usec: u64,
    pub user_usec: u64,
    pub system_usec: u64,
    pub throttled_periods: u64,
    pub throttled_usec: u64,
    pub nr_periods: u64,
    pub nr_throttled: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MemorySample {
    pub available: bool,
    pub current_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_max_bytes: Option<u64>,
    pub ooms: u64,
    pub oom_kills: u64,
    pub high_events: u64,
    pub low_events: u64,
    pub max_events: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IoSample {
    pub available: bool,
    pub rbytes: u64,
    pub wbytes: u64,
    pub rios: u64,
    pub wios: u64,
    pub dbytes: u64,
    pub dios: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PidsSample {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
    pub current: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PressureSample {
    pub cpu: PressureMetric,
    pub memory: PressureMetric,
    pub io: PressureMetric,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PressureMetric {
    pub available: bool,
    pub some_avg10: f64,
    pub some_avg60: f64,
    pub some_avg300: f64,
    pub some_total: u64,
    pub full_avg10: f64,
    pub full_avg60: f64,
    pub full_avg300: f64,
    pub full_total: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DiskSample {
    pub available: bool,
    pub state_directory: String,
    pub free_bytes: u64,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub reserved_bytes: u64,
    pub below_reserve: bool,
}

pub fn discover(
    configured_directory: Option<&Path>,
    state_directory: Option<&Path>,
) -> ResourceIdentity {
    let (directory, display_path) = match configured_directory {
        Some(directory) => (directory.to_path_buf(), directory.display().to_string()),
        None => self_cgroup_directory()
            .unwrap_or_else(|| (PathBuf::from("/sys/fs/cgroup"), "unresolved".into())),
    };
    let state_directory = state_directory.unwrap_or_else(|| Path::new("/var/lib/workflow-rust"));
    ResourceIdentity {
        cgroup_version: 2,
        cgroup_path: display_path,
        cpu: cpu_identity(&directory),
        memory: memory_identity(&directory),
        io: io_identity(&directory),
        pids: pids_identity(&directory),
        pressure: pressure_identity(&directory),
        disk: disk_identity(state_directory),
        accelerator: AcceleratorIdentity {
            policy: "off",
            eligible: false,
            reason: Some("Accelerator policy is Off by default; Safe Auto reports no eligible accelerator in the default bundle."),
        },
    }
}

/// Return the cgroup-v2 CPU usage counter in microseconds. The counter is
/// sampled around one native run, so it is only exposed as a run measurement
/// when the cgroup controller is readable.
pub fn cpu_usage_micros(configured_directory: Option<&Path>) -> Option<u64> {
    let directory = configured_directory
        .map(Path::to_path_buf)
        .or_else(|| self_cgroup_directory().map(|(directory, _)| directory))?;
    let stat = read(&directory, "cpu.stat")?;
    stat.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        (fields.next() == Some("usage_usec"))
            .then(|| fields.next()?.parse::<u64>().ok())
            .flatten()
    })
}

/// Read a point-in-time sample of every available cgroup controller plus the
/// state-disk free space. Unavailable controllers report available=false with
/// zeroed counters rather than erroring, so sampling never blocks execution.
pub fn sample(
    configured_directory: Option<&Path>,
    state_directory: Option<&Path>,
    reserved_disk_bytes: u64,
) -> ResourceSample {
    let directory = configured_directory
        .map(Path::to_path_buf)
        .or_else(|| self_cgroup_directory().map(|(directory, _)| directory));
    let state_directory = state_directory.unwrap_or_else(|| Path::new("/var/lib/workflow-rust"));
    let now = now_millis();
    let mut sample = ResourceSample {
        sampled_at_millis: now,
        ..Default::default()
    };
    if let Some(directory) = directory.as_deref() {
        sample.cpu = cpu_sample(directory);
        sample.memory = memory_sample(directory);
        sample.io = io_sample(directory);
        sample.pids = pids_sample(directory);
        sample.pressure = pressure_sample(directory);
    }
    sample.disk = disk_sample(state_directory, reserved_disk_bytes);
    sample
}

fn self_cgroup_directory() -> Option<(PathBuf, String)> {
    let membership = fs::read_to_string("/proc/self/cgroup").ok()?;
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?;
    let relative = relative.trim_start_matches('/');
    let directory = Path::new("/sys/fs/cgroup").join(relative);
    Some((directory, format!("/{relative}")))
}

fn cpu_identity(directory: &Path) -> ControllerIdentity {
    let Some(cpu_max) = read(directory, "cpu.max") else {
        return unavailable("cpu.max unavailable");
    };
    let mut values = BTreeMap::new();
    let fields: Vec<_> = cpu_max.split_whitespace().collect();
    if fields.len() == 2 {
        if let Ok(period) = fields[1].parse::<u64>() {
            values.insert("period_usec".into(), period.into());
            if fields[0] == "max" {
                values.insert("quota_cores".into(), serde_json::Value::Null);
            } else if let Ok(quota) = fields[0].parse::<u64>() {
                values.insert("quota_usec".into(), quota.into());
                values.insert("quota_cores".into(), (quota as f64 / period as f64).into());
            }
        }
    }
    if let Some(stat) = read(directory, "cpu.stat") {
        for line in stat.lines() {
            let mut fields = line.split_whitespace();
            if let (Some(name), Some(value)) = (fields.next(), fields.next()) {
                if let Ok(value) = value.parse::<u64>() {
                    values.insert(name.into(), value.into());
                }
            }
        }
    }
    ControllerIdentity {
        available: true,
        reason: None,
        values,
    }
}

fn memory_identity(directory: &Path) -> ControllerIdentity {
    let Some(current) = read_number_or_max(directory, "memory.current") else {
        return unavailable("memory.current unavailable");
    };
    let mut values = BTreeMap::new();
    values.insert("current_bytes".into(), current);
    for (file, key) in [
        ("memory.max", "max_bytes"),
        ("memory.high", "high_bytes"),
        ("memory.peak", "peak_bytes"),
        ("memory.swap.max", "swap_max_bytes"),
    ] {
        if let Some(value) = read_number_or_max(directory, file) {
            values.insert(key.into(), value);
        }
    }
    for (file, key) in [
        ("memory.events", "events"),
        ("memory.oom.group", "oom_group"),
    ] {
        if let Some(text) = read(directory, file) {
            values.insert(key.into(), parse_key_value_map(&text).into());
        }
    }
    ControllerIdentity {
        available: true,
        reason: None,
        values,
    }
}

fn io_identity(directory: &Path) -> ControllerIdentity {
    if let Some(stat) = read(directory, "io.stat") {
        ControllerIdentity {
            available: true,
            reason: None,
            values: parse_io_stat(&stat),
        }
    } else {
        unavailable("io.stat unavailable")
    }
}

fn pids_identity(directory: &Path) -> ControllerIdentity {
    let Some(maximum) = read_number_or_max(directory, "pids.max") else {
        return unavailable("pids.max unavailable");
    };
    let mut values = BTreeMap::new();
    values.insert("max".into(), maximum);
    if let Some(current) = read_number_or_max(directory, "pids.current") {
        values.insert("current".into(), current);
    }
    ControllerIdentity {
        available: true,
        reason: None,
        values,
    }
}

fn pressure_identity(directory: &Path) -> ControllerIdentity {
    let mut values = BTreeMap::new();
    for (file, key) in [
        ("cpu.pressure", "cpu"),
        ("memory.pressure", "memory"),
        ("io.pressure", "io"),
    ] {
        if let Some(text) = read(directory, file) {
            values.insert(key.into(), parse_pressure_file(&text).into());
        }
    }
    if values.is_empty() {
        return unavailable("PSI files unavailable");
    }
    ControllerIdentity {
        available: true,
        reason: None,
        values,
    }
}

fn disk_identity(state_directory: &Path) -> DiskIdentity {
    let mut values = BTreeMap::new();
    match disk_stats(state_directory) {
        Ok((free, total, available)) => {
            values.insert("free_bytes".into(), free.into());
            values.insert("total_bytes".into(), total.into());
            values.insert("available_bytes".into(), available.into());
            values.insert(
                "managed_reserve_bytes".into(),
                DEFAULT_MANAGED_DISK_RESERVE_BYTES.into(),
            );
            DiskIdentity {
                available: true,
                reason: None,
                state_directory: state_directory.display().to_string(),
                values,
            }
        }
        Err(reason) => DiskIdentity {
            available: false,
            reason: Some(reason),
            state_directory: state_directory.display().to_string(),
            values,
        },
    }
}

fn cpu_sample(directory: &Path) -> CpuSample {
    let mut sample = CpuSample::default();
    if let Some(cpu_max) = read(directory, "cpu.max") {
        let fields: Vec<_> = cpu_max.split_whitespace().collect();
        if fields.len() == 2 {
            if let Ok(period) = fields[1].parse::<u64>() {
                sample.period_usec = Some(period);
                if fields[0] == "max" {
                    sample.quota_cores = None;
                } else if let Ok(quota) = fields[0].parse::<u64>() {
                    sample.quota_cores = Some(quota as f64 / period as f64);
                }
            }
        }
        sample.available = true;
    }
    if let Some(text) = read(directory, "cpu.stat") {
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let Some(name) = fields.next() else { continue };
            let Some(value) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                continue;
            };
            match name {
                "usage_usec" => sample.usage_usec = value,
                "user_usec" => sample.user_usec = value,
                "system_usec" => sample.system_usec = value,
                "nr_periods" => sample.nr_periods = value,
                "nr_throttled" => sample.nr_throttled = value,
                "throttled_usec" => sample.throttled_usec = value,
                "throttled_periods" => sample.throttled_periods = value,
                _ => {}
            }
        }
        sample.available = true;
    }
    sample
}

fn memory_sample(directory: &Path) -> MemorySample {
    let mut sample = MemorySample::default();
    if let Some(current) = read_number(directory, "memory.current") {
        sample.current_bytes = current;
        sample.available = true;
    }
    sample.max_bytes = read_number_or_max(directory, "memory.max").and_then(value_to_u64);
    sample.high_bytes = read_number_or_max(directory, "memory.high").and_then(value_to_u64);
    sample.peak_bytes = read_number(directory, "memory.peak");
    sample.swap_max_bytes = read_number_or_max(directory, "memory.swap.max").and_then(value_to_u64);
    if let Some(text) = read(directory, "memory.events") {
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let Some(name) = fields.next() else { continue };
            let Some(value) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                continue;
            };
            match name {
                "oom" => sample.ooms = value,
                "oom_kill" => sample.oom_kills = value,
                "high" => sample.high_events = value,
                "low" => sample.low_events = value,
                "max" => sample.max_events = value,
                _ => {}
            }
        }
    }
    sample
}

fn io_sample(directory: &Path) -> IoSample {
    let mut sample = IoSample::default();
    let Some(text) = read(directory, "io.stat") else {
        return sample;
    };
    sample.available = true;
    for (_device, fields) in parse_io_per_device(&text) {
        for (name, value) in fields {
            match name.as_str() {
                "rbytes" => sample.rbytes = sample.rbytes.saturating_add(value),
                "wbytes" => sample.wbytes = sample.wbytes.saturating_add(value),
                "rios" => sample.rios = sample.rios.saturating_add(value),
                "wios" => sample.wios = sample.wios.saturating_add(value),
                "dbytes" => sample.dbytes = sample.dbytes.saturating_add(value),
                "dios" => sample.dios = sample.dios.saturating_add(value),
                _ => {}
            }
        }
    }
    sample
}

fn pids_sample(directory: &Path) -> PidsSample {
    let mut sample = PidsSample::default();
    if let Some(maximum) = read_number_or_max(directory, "pids.max") {
        sample.max = value_to_u64(maximum);
        sample.available = true;
    }
    if let Some(current) = read_number(directory, "pids.current") {
        sample.current = current;
        sample.available = true;
    }
    sample
}

fn pressure_sample(directory: &Path) -> PressureSample {
    PressureSample {
        cpu: pressure_metric(directory, "cpu.pressure"),
        memory: pressure_metric(directory, "memory.pressure"),
        io: pressure_metric(directory, "io.pressure"),
    }
}

fn pressure_metric(directory: &Path, name: &str) -> PressureMetric {
    let Some(text) = read(directory, name) else {
        return PressureMetric::default();
    };
    let mut metric = PressureMetric {
        available: true,
        ..Default::default()
    };
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(kind) = fields.next() else { continue };
        for field in fields {
            let Some((key, raw)) = field.split_once('=') else {
                continue;
            };
            let parsed = if key == "total" {
                raw.parse::<u64>().ok().map(|v| v as f64)
            } else {
                raw.parse::<f64>().ok()
            };
            let Some(value) = parsed else { continue };
            match (kind, key) {
                ("some", "avg10") => metric.some_avg10 = value,
                ("some", "avg60") => metric.some_avg60 = value,
                ("some", "avg300") => metric.some_avg300 = value,
                ("some", "total") => metric.some_total = value as u64,
                ("full", "avg10") => metric.full_avg10 = value,
                ("full", "avg60") => metric.full_avg60 = value,
                ("full", "avg300") => metric.full_avg300 = value,
                ("full", "total") => metric.full_total = value as u64,
                _ => {}
            }
        }
    }
    metric
}

fn disk_sample(directory: &Path, reserved_bytes: u64) -> DiskSample {
    let mut sample = DiskSample {
        state_directory: directory.display().to_string(),
        reserved_bytes,
        ..Default::default()
    };
    match disk_stats(directory) {
        Ok((free_bytes, total_bytes, available_bytes)) => {
            sample.available = true;
            sample.free_bytes = free_bytes;
            sample.total_bytes = total_bytes;
            sample.available_bytes = available_bytes;
            sample.below_reserve = available_bytes < reserved_bytes;
        }
        Err(_) => {
            sample.available = false;
        }
    }
    sample
}

/// Query filesystem free space using POSIX `df -Pk`. This avoids adding a libc
/// or `fs2` dependency and is called infrequently (see `DISK_SAMPLE_INTERVAL`)
/// so the fork+exec cost is negligible. Returns (free_bytes, total_bytes,
/// available_bytes) in bytes.
fn disk_stats(directory: &Path) -> Result<(u64, u64, u64), String> {
    use std::process::Command;
    let output = Command::new("df")
        .args(["-Pk", "."])
        .current_dir(directory)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("df -Pk unavailable".into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // df -Pk header: Filesystem 1024-blocks Used Available Capacity Mounted on
    let data = stdout
        .lines()
        .nth(1)
        .ok_or_else(|| "df produced no data row".to_string())?;
    let mut fields = data.split_whitespace();
    let _filesystem = fields.next();
    let blocks_kb: u64 = fields
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| "df blocks unavailable".to_string())?;
    let _used_kb = fields.next();
    let available_kb: u64 = fields
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| "df available blocks unavailable".to_string())?;
    let total = blocks_kb.saturating_mul(1024);
    let available = available_kb.saturating_mul(1024);
    Ok((available, total, available))
}

fn unavailable(reason: &str) -> ControllerIdentity {
    ControllerIdentity {
        available: false,
        reason: Some(reason.into()),
        values: BTreeMap::new(),
    }
}

fn read(directory: &Path, name: &str) -> Option<String> {
    fs::read_to_string(directory.join(name))
        .ok()
        .map(|value| value.trim().to_owned())
}

fn read_number(directory: &Path, name: &str) -> Option<u64> {
    read(directory, name)?.parse::<u64>().ok()
}

fn read_number_or_max(directory: &Path, name: &str) -> Option<serde_json::Value> {
    let value = read(directory, name)?;
    if value == "max" {
        Some(serde_json::Value::Null)
    } else {
        value.parse::<u64>().ok().map(Into::into)
    }
}

fn value_to_u64(value: serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::Number(number) => number.as_u64(),
        _ => None,
    }
}

fn parse_key_value_map(text: &str) -> BTreeMap<String, serde_json::Value> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let (Some(name), Some(raw)) = (fields.next(), fields.next()) else {
            continue;
        };
        if let Ok(value) = raw.parse::<u64>() {
            values.insert(name.into(), value.into());
        }
    }
    values
}

fn parse_pressure_file(text: &str) -> serde_json::Value {
    let mut metric = PressureMetric {
        available: true,
        ..Default::default()
    };
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(kind) = fields.next() else { continue };
        for field in fields {
            let Some((key, raw)) = field.split_once('=') else {
                continue;
            };
            let parsed = if key == "total" {
                raw.parse::<u64>().ok().map(|v| v as f64)
            } else {
                raw.parse::<f64>().ok()
            };
            let Some(value) = parsed else { continue };
            match (kind, key) {
                ("some", "avg10") => metric.some_avg10 = value,
                ("some", "avg60") => metric.some_avg60 = value,
                ("some", "avg300") => metric.some_avg300 = value,
                ("some", "total") => metric.some_total = value as u64,
                ("full", "avg10") => metric.full_avg10 = value,
                ("full", "avg60") => metric.full_avg60 = value,
                ("full", "avg300") => metric.full_avg300 = value,
                ("full", "total") => metric.full_total = value as u64,
                _ => {}
            }
        }
    }
    serde_json::to_value(metric).unwrap_or(serde_json::Value::Null)
}

fn parse_io_stat(text: &str) -> BTreeMap<String, serde_json::Value> {
    let mut totals: BTreeMap<String, u64> = BTreeMap::new();
    for (_device, fields) in parse_io_per_device(text) {
        for (name, value) in fields {
            *totals.entry(name).or_insert(0) += value;
        }
    }
    totals
        .into_iter()
        .map(|(key, value)| (key, value.into()))
        .collect()
}

fn parse_io_per_device(text: &str) -> Vec<(String, BTreeMap<String, u64>)> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(device) = fields.next() else { continue };
        if !device.contains(':') {
            continue;
        }
        let mut values = BTreeMap::new();
        for field in fields {
            let Some((name, raw)) = field.split_once('=') else {
                continue;
            };
            if let Ok(value) = raw.parse::<u64>() {
                values.insert(name.into(), value);
            }
        }
        rows.push((device.into(), values));
    }
    rows
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tmp(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "canopy-{name}-{}-{}",
            std::process::id(),
            std::thread::current().id().as_u64()
        ));
        std::fs::create_dir_all(&path).expect("tempdir created");
        path
    }

    #[test]
    fn parses_cpu_max_quota_cores() {
        let tmp = make_tmp("cpu-quota");
        std::fs::write(tmp.join("cpu.max"), "50000 100000").unwrap();
        std::fs::write(
            tmp.join("cpu.stat"),
            "usage_usec 1234\nuser_usec 1000\nsystem_usec 234\nnr_periods 5\nnr_throttled 2\nthrottled_usec 100\nthrottled_periods 2\n",
        ).unwrap();
        let identity = cpu_identity(&tmp);
        assert!(identity.available);
        assert_eq!(identity.values["quota_cores"], serde_json::json!(0.5));
        assert_eq!(identity.values["throttled_usec"], serde_json::json!(100));
        let sample = cpu_sample(&tmp);
        assert!(sample.available);
        assert_eq!(sample.quota_cores, Some(0.5));
        assert_eq!(sample.usage_usec, 1234);
        assert_eq!(sample.throttled_usec, 100);
    }

    #[test]
    fn parses_memory_events() {
        let tmp = make_tmp("mem-events");
        std::fs::write(tmp.join("memory.current"), "1048576").unwrap();
        std::fs::write(tmp.join("memory.max"), "max").unwrap();
        std::fs::write(
            tmp.join("memory.events"),
            "low 0\nhigh 3\nmax 1\noom 0\noom_kill 0\n",
        )
        .unwrap();
        let sample = memory_sample(&tmp);
        assert!(sample.available);
        assert_eq!(sample.current_bytes, 1_048_576);
        assert_eq!(sample.high_events, 3);
        assert_eq!(sample.max_events, 1);
    }

    #[test]
    fn parses_io_stat_aggregated() {
        let tmp = make_tmp("io-stat");
        std::fs::write(
            tmp.join("io.stat"),
            "8:0 rbytes=100 wbytes=200 rios=5 wios=6 dbytes=0 dios=0\n8:1 rbytes=10 wbytes=20 rios=1 wios=1\n",
        ).unwrap();
        let sample = io_sample(&tmp);
        assert!(sample.available);
        assert_eq!(sample.rbytes, 110);
        assert_eq!(sample.wbytes, 220);
        assert_eq!(sample.rios, 6);
        assert_eq!(sample.wios, 7);
    }

    #[test]
    fn parses_pressure_stall_information() {
        let tmp = make_tmp("psi");
        std::fs::write(
            tmp.join("memory.pressure"),
            "some avg10=1.23 avg60=0.45 avg300=0.12 total=12345\nfull avg10=0.10 avg60=0.02 avg300=0.01 total=500\n",
        ).unwrap();
        let metric = pressure_metric(&tmp, "memory.pressure");
        assert!(metric.available);
        assert!((metric.some_avg10 - 1.23).abs() < f64::EPSILON);
        assert_eq!(metric.some_total, 12345);
        assert!((metric.full_avg10 - 0.10).abs() < f64::EPSILON);
    }

    #[test]
    fn unavailable_controller_reports_safe_zero_sample() {
        let tmp = make_tmp("unavail");
        let sample = cpu_sample(&tmp);
        assert!(!sample.available);
        assert_eq!(sample.usage_usec, 0);
        let mem = memory_sample(&tmp);
        assert!(!mem.available);
        assert_eq!(mem.current_bytes, 0);
    }

    trait ThreadIdExt {
        fn as_u64(&self) -> u64;
    }
    impl ThreadIdExt for std::thread::ThreadId {
        fn as_u64(&self) -> u64 {
            let rendered = format!("{self:?}");
            rendered
                .trim_start_matches("ThreadId(")
                .trim_end_matches(')')
                .parse::<u64>()
                .unwrap_or(0)
        }
    }
}
