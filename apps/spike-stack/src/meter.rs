//! PROTOTYPE: CPU and memory of this process and every process it started
//! (WebView2's browser, GPU and renderer processes), so both stacks are measured
//! the same way.

use std::time::Instant;

use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub processes: usize,
    /// Kernel + user CPU of the whole tree, in seconds.
    pub cpu_s: f64,
    pub private_mb: f64,
    pub working_set_mb: f64,
}

pub struct Meter {
    start: Instant,
    start_cpu: f64,
    last: Instant,
    last_cpu: f64,
    pub peak_private_mb: f64,
    pub peak_working_set_mb: f64,
    pub peak_processes: usize,
}

pub struct Report {
    pub now: Sample,
    /// Percent of one core since the previous report.
    pub interval_pct: f64,
    /// Percent of one core since the meter started.
    pub overall_pct: f64,
}

impl Meter {
    pub fn new() -> Self {
        let s = sample();
        let now = Instant::now();
        Self {
            start: now,
            start_cpu: s.cpu_s,
            last: now,
            last_cpu: s.cpu_s,
            peak_private_mb: s.private_mb,
            peak_working_set_mb: s.working_set_mb,
            peak_processes: s.processes,
        }
    }

    /// Restarts the averages, so start-up work is not counted.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn report(&mut self) -> Report {
        let s = sample();
        let now = Instant::now();
        let interval_pct =
            100.0 * (s.cpu_s - self.last_cpu) / now.duration_since(self.last).as_secs_f64();
        let overall_pct =
            100.0 * (s.cpu_s - self.start_cpu) / now.duration_since(self.start).as_secs_f64();
        self.last = now;
        self.last_cpu = s.cpu_s;
        self.peak_private_mb = self.peak_private_mb.max(s.private_mb);
        self.peak_working_set_mb = self.peak_working_set_mb.max(s.working_set_mb);
        self.peak_processes = self.peak_processes.max(s.processes);
        Report { now: s, interval_pct, overall_pct }
    }
}

fn filetime_s(t: FILETIME) -> f64 {
    ((u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)) as f64 / 1e7
}

/// This process and its descendants.
fn tree() -> Vec<u32> {
    let me = unsafe { GetCurrentProcessId() };
    let mut all = Vec::new();
    // SAFETY: plain Toolhelp enumeration; the entry is sized as the API requires.
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return vec![me];
        };
        let mut e = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                all.push((e.th32ProcessID, e.th32ParentProcessID));
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    let mut out = vec![me];
    let mut i = 0;
    while i < out.len() {
        let parent = out[i];
        for &(pid, ppid) in &all {
            if ppid == parent && pid != parent && !out.contains(&pid) {
                out.push(pid);
            }
        }
        i += 1;
    }
    out
}

pub fn sample() -> Sample {
    let mut s = Sample::default();
    for pid in tree() {
        // SAFETY: query-only handle, closed below; out-params are valid locals.
        unsafe {
            let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                continue;
            };
            let (mut c, mut x, mut k, mut u) = Default::default();
            if GetProcessTimes(h, &mut c, &mut x, &mut k, &mut u).is_ok() {
                s.cpu_s += filetime_s(k) + filetime_s(u);
            }
            let mut m = PROCESS_MEMORY_COUNTERS_EX {
                cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
                ..Default::default()
            };
            if GetProcessMemoryInfo(
                h,
                (&raw mut m).cast::<PROCESS_MEMORY_COUNTERS>(),
                m.cb,
            )
            .is_ok()
            {
                s.private_mb += m.PrivateUsage as f64 / 1048576.0;
                s.working_set_mb += m.WorkingSetSize as f64 / 1048576.0;
            }
            s.processes += 1;
            let _ = CloseHandle(h);
        }
    }
    s
}

/// Total CPU seconds of every process with this executable name (e.g. `dwm.exe`),
/// or `None` if none could be opened.
pub fn named_cpu_s(exe: &str) -> Option<f64> {
    let mut total = None;
    // SAFETY: Toolhelp enumeration and query-only handles, closed after use.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut e = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                if String::from_utf16_lossy(&e.szExeFile[..len]).eq_ignore_ascii_case(exe)
                    && let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, e.th32ProcessID)
                {
                    let (mut c, mut x, mut k, mut u) = Default::default();
                    if GetProcessTimes(h, &mut c, &mut x, &mut k, &mut u).is_ok() {
                        *total.get_or_insert(0.0) += filetime_s(k) + filetime_s(u);
                    }
                    let _ = CloseHandle(h);
                }
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    total
}
