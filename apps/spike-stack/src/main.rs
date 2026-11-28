//! PROTOTYPE for wayfinder ticket #28 (stack spike). Throwaway: lives only on
//! the `prototype/28-stack-spike` branch and is never merged.
//!
//! Draws the Relative as `--widgets` copies, natively (Direct2D +
//! DirectComposition) or in WebView2, one window per widget or one composition
//! surface per monitor. A feed thread polls the sim (or a tape), computes the
//! Relative once per tick and hands the latest snapshot to the render thread,
//! which wakes once per snapshot. The process runs at below-normal priority
//! unless `--normal-priority`. Every 10 s and at the end it prints the CPU and
//! memory of its whole process tree (WebView2's child processes included) and
//! the render thread's cost and freshness.

#[path = "../../reo-cli/src/irdt.rs"]
#[allow(dead_code)]
mod irdt;
mod meter;
mod native;
mod relative;
mod web;
mod win;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use reo_iracing::{Connection, IRacingSource};
use reo_irsdk::{Client, SharedBuffer};
use reo_model::{Poll, Telemetry, TelemetrySource};
use windows::Win32::Foundation::{HANDLE, RECT, WAIT_OBJECT_0};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::Threading::{
    BELOW_NORMAL_PRIORITY_CLASS, CreateEventW, GetCurrentProcess, SetEvent, SetPriorityClass,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, MsgWaitForMultipleObjects, PM_REMOVE, PeekMessageW, QS_ALLINPUT,
    TranslateMessage,
};

use crate::irdt::{IrdtTape, Record};
use crate::relative::Snap;

pub const WIDGET_W: i32 = 380;
pub const WIDGET_H: i32 = 16 + relative::ROWS as i32 * 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Stack {
    /// Feed and Relative only, nothing drawn: the baseline.
    None,
    Native,
    Webview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Layout {
    PerWidget,
    PerMonitor,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, value_enum, default_value_t = Stack::Native)]
    stack: Stack,
    #[arg(long, value_enum, default_value_t = Layout::PerWidget)]
    layout: Layout,
    /// Copies of the Relative, spread across the monitors.
    #[arg(long, default_value_t = 6)]
    widgets: usize,
    /// Replay an irDashies `.irdt` tape instead of the live sim (loops).
    #[arg(long)]
    irdt: Option<PathBuf>,
    #[arg(long, default_value_t = 1.0)]
    speed: f64,
    /// Fast-forward this many tape seconds before pacing starts.
    #[arg(long, default_value_t = 0.0)]
    skip: f64,
    /// Measure for this long after a 5 s warm-up, then exit.
    #[arg(long, default_value_t = 60)]
    seconds: u64,
    /// Run at normal priority instead of below normal.
    #[arg(long)]
    normal_priority: bool,
    /// Show real driver names (default: "Driver <number>", so screenshots carry no names).
    #[arg(long)]
    names: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub monitor: RECT,
    pub x: i32,
    pub y: i32,
}

pub trait Renderer {
    fn draw(&mut self, snap: &Snap, names: bool) -> Result<()>;
    fn stats(&self) -> String {
        String::new()
    }
}

/// CPU seconds (kernel + user) the calling thread has used.
fn thread_cpu_s() -> f64 {
    use windows::Win32::System::Threading::{GetCurrentThread, GetThreadTimes};
    let (mut c, mut e, mut k, mut u) = Default::default();
    // SAFETY: pseudo-handle for this thread; out-params are valid locals.
    unsafe {
        let _ = GetThreadTimes(GetCurrentThread(), &mut c, &mut e, &mut k, &mut u);
    }
    let s = |t: windows::Win32::Foundation::FILETIME| {
        ((u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)) as f64 / 1e7
    };
    s(k) + s(u)
}

struct Shared {
    snap: Mutex<Snap>,
    event: isize,
    done: AtomicBool,
    /// The feed thread's CPU seconds so far, as f64 bits.
    feed_cpu: AtomicU64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    // SAFETY: process-wide settings made once, before any window exists.
    let event = unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if !args.normal_priority {
            // Child processes (WebView2's) inherit below-normal from us.
            SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS)?;
        }
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        CreateEventW(None, false, false, None)?
    };
    let shared = Arc::new(Shared {
        snap: Mutex::new(Snap::default()),
        event: event.0 as isize,
        done: AtomicBool::new(false),
        feed_cpu: AtomicU64::new(0),
    });

    let monitors = win::monitors();
    let placements: Vec<Placement> = (0..args.widgets)
        .map(|i| {
            let m = monitors[i % monitors.len()];
            let slot = (i / monitors.len()) as i32;
            Placement {
                monitor: m,
                x: m.left + 40,
                y: m.top + 80 + slot * (WIDGET_H + 20),
            }
        })
        .collect();

    let feed = {
        let shared = shared.clone();
        let irdt = args.irdt.clone();
        let (speed, skip) = (args.speed, args.skip);
        std::thread::Builder::new()
            .name("feed".into())
            .spawn(move || run_feed(irdt, speed, skip, &shared))?
    };

    let mut renderer: Option<Box<dyn Renderer>> = match args.stack {
        Stack::None => None,
        Stack::Native => Some(Box::new(native::Native::new(&placements, args.layout)?)),
        Stack::Webview => Some(Box::new(web::Web::new(&placements, args.layout)?)),
    };

    println!(
        "stack={:?} layout={:?} widgets={} monitors={} priority={}",
        args.stack,
        args.layout,
        args.widgets,
        monitors.len(),
        if args.normal_priority { "normal" } else { "below-normal" }
    );

    let warmup = Duration::from_secs(5);
    let started = Instant::now();
    let mut measuring = false;
    let mut meter = meter::Meter::new();
    let mut next_report = started + warmup + Duration::from_secs(10);
    let mut snap = Snap::default();
    let mut last_seq = 0;
    let cap = (args.seconds as usize + 10) * 120;
    let mut draw_us: Vec<f32> = Vec::with_capacity(cap);
    let mut fresh_us: Vec<f32> = Vec::with_capacity(cap);
    let mut msg = MSG::default();
    let (mut render_cpu0, mut feed_cpu0, mut dwm_cpu0) = (0.0, 0.0, None);
    let handle = HANDLE(shared.event as *mut _);

    loop {
        // SAFETY: waiting on our own event and this thread's message queue.
        let w = unsafe { MsgWaitForMultipleObjects(Some(&[handle]), false, 200, QS_ALLINPUT) };
        if w == WAIT_OBJECT_0 {
            snap.clone_from(&shared.snap.lock().unwrap());
            if snap.seq != last_seq {
                last_seq = snap.seq;
                if let Some(r) = renderer.as_mut() {
                    let t0 = Instant::now();
                    r.draw(&snap, args.names)?;
                    if measuring {
                        draw_us.push(t0.elapsed().as_secs_f32() * 1e6);
                        fresh_us.push(snap.published.elapsed().as_secs_f32() * 1e6);
                    }
                }
            }
        } else if w.0 == WAIT_OBJECT_0.0 + 1 {
            // SAFETY: standard message pump on the thread that owns the windows.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        let now = Instant::now();
        if !measuring && now >= started + warmup {
            measuring = true;
            meter.reset();
            render_cpu0 = thread_cpu_s();
            feed_cpu0 = f64::from_bits(shared.feed_cpu.load(Ordering::Relaxed));
            dwm_cpu0 = meter::named_cpu_s("dwm.exe");
        }
        if measuring && now >= next_report {
            let r = meter.report();
            println!(
                "  {:>4.0}s  cpu {:>5.2}% of a core  private {:>6.1} MB  working set {:>6.1} MB  processes {}",
                (now - started - warmup).as_secs_f64(),
                r.interval_pct,
                r.now.private_mb,
                r.now.working_set_mb,
                r.now.processes
            );
            next_report = now + Duration::from_secs(10);
        }
        if now >= started + warmup + Duration::from_secs(args.seconds) {
            break;
        }
    }

    let r = meter.report();
    let secs = args.seconds as f64;
    let render_pct = 100.0 * (thread_cpu_s() - render_cpu0) / secs;
    let feed_pct = 100.0 * (f64::from_bits(shared.feed_cpu.load(Ordering::Relaxed)) - feed_cpu0) / secs;
    let dwm_pct = match (dwm_cpu0, meter::named_cpu_s("dwm.exe")) {
        (Some(a), Some(b)) => format!("{:.2}%", 100.0 * (b - a) / secs),
        _ => "n/a".into(),
    };
    shared.done.store(true, Ordering::Relaxed);
    println!("RESULT stack={:?} layout={:?} widgets={}", args.stack, args.layout, args.widgets);
    println!("  cpu (whole tree)  {:.2}% of one core over {} s", r.overall_pct, args.seconds);
    println!(
        "  memory            private {:.1} MB (peak {:.1}), working set {:.1} MB (peak {:.1}), {} processes",
        r.now.private_mb, meter.peak_private_mb, r.now.working_set_mb, meter.peak_working_set_mb, meter.peak_processes
    );
    println!("  frames drawn      {} ({:.1}/s)", draw_us.len(), draw_us.len() as f64 / args.seconds as f64);
    print_dist("  render cpu/frame ", &mut draw_us);
    print_dist("  tick->drawn      ", &mut fresh_us);
    println!("  threads           render {render_pct:.2}%, feed {feed_pct:.2}%, rest of tree {:.2}% (of one core)", r.overall_pct - render_pct - feed_pct);
    println!("  dwm.exe           {dwm_pct} of one core (whole desktop, not only us)");
    if let Some(r) = renderer.as_ref() {
        println!("  renderer          {}", r.stats());
    }
    drop(renderer.take());
    let _ = feed.join();
    Ok(())
}

fn print_dist(label: &str, v: &mut [f32]) {
    if v.is_empty() {
        return;
    }
    v.sort_unstable_by(f32::total_cmp);
    let q = |p: f64| v[((v.len() - 1) as f64 * p) as usize];
    let mean = v.iter().map(|x| f64::from(*x)).sum::<f64>() / v.len() as f64;
    println!(
        "{label} mean {mean:.0} us  p50 {:.0}  p99 {:.0}  max {:.0}",
        q(0.5),
        q(0.99),
        v[v.len() - 1]
    );
}

fn tape_source(mem: SharedBuffer) -> IRacingSource<SharedBuffer> {
    IRacingSource::with_opener(move || {
        Some(Connection {
            client: Client::new(mem.clone()),
            waiter: None,
        })
    })
}

fn run_feed(irdt: Option<PathBuf>, speed: f64, skip: f64, shared: &Shared) {
    let mut t = Telemetry::default();
    let mut snap = Snap::default();
    let mut scratch = Vec::with_capacity(64);
    let publish = |t: &Telemetry, src: &dyn TelemetrySource, snap: &mut Snap, scratch: &mut Vec<_>| {
        let session = src.session();
        relative::compute(t, session.as_deref(), snap, scratch);
        snap.session = session;
        snap.seq += 1;
        snap.published = Instant::now();
        shared.feed_cpu.store(thread_cpu_s().to_bits(), Ordering::Relaxed);
        shared.snap.lock().unwrap().clone_from(snap);
        // SAFETY: our own event handle, alive for the process.
        unsafe {
            let _ = SetEvent(HANDLE(shared.event as *mut _));
        }
    };

    match irdt {
        None => {
            let mut src = IRacingSource::live();
            while !shared.done.load(Ordering::Relaxed) {
                // Blocks on the sim's data-ready event: no polling timer.
                if let Poll::Updated { .. } = src.poll(Duration::from_millis(200), &mut t) {
                    publish(&t, &src, &mut snap, &mut scratch);
                }
            }
        }
        Some(path) => 'tape: loop {
            let Ok(mut tape) = IrdtTape::open(&path) else {
                eprintln!("cannot open tape");
                return;
            };
            let mut src = tape_source(tape.memory());
            let start = Instant::now();
            loop {
                if shared.done.load(Ordering::Relaxed) {
                    break 'tape;
                }
                match tape.next_record() {
                    Ok(None | Some(Record::End)) | Err(_) => continue 'tape,
                    Ok(Some(Record::Frame { elapsed_s })) => {
                        if elapsed_s > skip {
                            let due = start + Duration::from_secs_f64((elapsed_s - skip) / speed);
                            if let Some(wait) = due.checked_duration_since(Instant::now()) {
                                std::thread::sleep(wait);
                            }
                        }
                        if let Poll::Updated { .. } = src.poll(Duration::ZERO, &mut t) {
                            publish(&t, &src, &mut snap, &mut scratch);
                        }
                    }
                    Ok(Some(_)) => {}
                }
            }
        },
    }
}
