//! `reo`: developer CLI for inspecting telemetry and checking the core against recordings.

mod irdt;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use reo_iracing::{Connection, IRacingSource};
use reo_irsdk::{Client, SharedBuffer, SharedMemory, VarTable};
use reo_model::{Poll, SessionInfo, Telemetry, TelemetrySource};

use crate::irdt::{IrdtTape, Record};

#[derive(Parser)]
#[command(
    name = "reo",
    version,
    about = "Inspect live or recorded sim telemetry"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print a live summary line a few times per second.
    Watch {
        #[command(flatten)]
        input: Input,
        /// Tape playback speed multiplier.
        #[arg(long, default_value_t = 1.0)]
        speed: f64,
    },
    /// List every variable the sim publishes, with its current value.
    Vars {
        #[command(flatten)]
        input: Input,
    },
    /// Print the parsed session info (or the raw YAML with --raw).
    Session {
        #[command(flatten)]
        input: Input,
        #[arg(long)]
        raw: bool,
    },
    /// Run a whole tape through the iRacing adapter as fast as possible and
    /// report timings and sanity statistics.
    Check {
        /// Path to an irDashies `.irdt` tape.
        tape: PathBuf,
    },
}

#[derive(Args, Clone)]
struct Input {
    /// Read an irDashies `.irdt` tape instead of the live sim.
    #[arg(long, conflicts_with = "mapping")]
    irdt: Option<PathBuf>,
    /// Shared-memory mapping name (default: iRacing's).
    #[arg(long)]
    mapping: Option<String>,
    /// Data-valid event name (default: iRacing's).
    #[arg(long, requires = "mapping")]
    event: Option<String>,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Watch { input, speed } => watch(&input, speed),
        Command::Vars { input } => vars(&input),
        Command::Session { input, raw } => session(&input, raw),
        Command::Check { tape } => check(&tape),
    }
}

/// Either a live connection or a tape that has to be stepped record by record.
/// Built once per command, so the tape variant's size doesn't matter.
#[allow(clippy::large_enum_variant)]
enum Feed {
    Live(Box<dyn LiveSource>),
    Tape(IrdtTape, IRacingSource<SharedBuffer>),
}

/// The parts of a live source the CLI needs beyond [`TelemetrySource`].
trait LiveSource: TelemetrySource {
    fn vars(&self) -> Option<std::sync::Arc<VarTable>>;
    fn raw_row(&self) -> Vec<u8>;
    fn session_yaml(&self) -> Option<String>;
    fn last_error(&self) -> Option<String>;
}

impl<M: SharedMemory + Send> LiveSource for IRacingSource<M> {
    fn vars(&self) -> Option<std::sync::Arc<VarTable>> {
        IRacingSource::vars(self).cloned()
    }
    fn raw_row(&self) -> Vec<u8> {
        self.raw_frame().row.clone()
    }
    fn session_yaml(&self) -> Option<String> {
        IRacingSource::session_yaml(self).map(str::to_owned)
    }
    fn last_error(&self) -> Option<String> {
        IRacingSource::last_error(self).map(str::to_owned)
    }
}

impl Feed {
    fn open(input: &Input) -> Result<Self> {
        if let Some(path) = &input.irdt {
            let tape = IrdtTape::open(path)?;
            let source = tape_source(tape.memory());
            return Ok(Self::Tape(tape, source));
        }
        live_feed(input)
    }

    fn source(&self) -> &dyn LiveSource {
        match self {
            Self::Live(s) => s.as_ref(),
            Self::Tape(_, s) => s,
        }
    }

    /// Blocks until the next telemetry tick. For tapes, `pace` sleeps to keep
    /// real time at the given speed. Returns `false` at the end of a tape.
    fn next(&mut self, out: &mut Telemetry, pace: Option<(Instant, f64)>) -> Result<bool> {
        match self {
            Self::Live(src) => loop {
                if let Poll::Updated { .. } = src.poll(Duration::from_millis(100), out) {
                    return Ok(true);
                }
            },
            Self::Tape(tape, src) => loop {
                match tape.next_record()? {
                    None | Some(Record::End) => return Ok(false),
                    Some(Record::Frame { elapsed_s }) => {
                        if let Some((start, speed)) = pace {
                            let due = start + Duration::from_secs_f64(elapsed_s / speed);
                            if let Some(wait) = due.checked_duration_since(Instant::now()) {
                                std::thread::sleep(wait);
                            }
                        }
                        if let Poll::Updated { .. } = src.poll(Duration::ZERO, out) {
                            return Ok(true);
                        }
                    }
                    Some(_) => {}
                }
            },
        }
    }
}

#[cfg(windows)]
fn live_feed(input: &Input) -> Result<Feed> {
    let source = match (&input.mapping, &input.event) {
        (Some(m), Some(e)) => IRacingSource::named(m, e),
        (Some(m), None) => IRacingSource::named(m, reo_irsdk::IRACING_EVENT_NAME),
        _ => IRacingSource::live(),
    };
    eprintln!("waiting for the sim...");
    Ok(Feed::Live(Box::new(source)))
}

#[cfg(not(windows))]
fn live_feed(_: &Input) -> Result<Feed> {
    bail!("live telemetry needs Windows; pass --irdt <tape>")
}

fn tape_source(mem: SharedBuffer) -> IRacingSource<SharedBuffer> {
    IRacingSource::with_opener(move || {
        Some(Connection {
            client: Client::new(mem.clone()),
            waiter: None,
        })
    })
}

fn watch(input: &Input, speed: f64) -> Result<()> {
    let mut feed = Feed::open(input)?;
    let mut t = Telemetry::default();
    let start = Instant::now();
    let pace = matches!(feed, Feed::Tape(..)).then_some((start, speed));
    let mut next_print = Instant::now();
    while feed.next(&mut t, pace)? {
        if Instant::now() >= next_print {
            println!("{}", summary_line(&t, feed.source().session().as_deref()));
            next_print = Instant::now() + Duration::from_millis(200);
        }
    }
    Ok(())
}

fn summary_line(t: &Telemetry, info: Option<&SessionInfo>) -> String {
    let p = &t.player;
    let in_world = t.cars.iter().filter(|c| c.surface.is_in_world()).count();
    let flags: Vec<_> = t.session.flags.names().collect();
    let driver = p
        .car_idx
        .and_then(|i| info.and_then(|s| s.driver(i)))
        .map_or("-", |d| d.name.as_str());
    format!(
        "t={:7.1}s tick {:>6} | {:<18} | {:5.1} km/h {:5.0} rpm G{:<2} | thr {:3.0}% brk {:3.0}% clu {:3.0}% | lap {:>2} {:5.1}% | fuel {:>5} | Δbest {:>6} | cars {:>2} | {:?} [{}]",
        t.session_time,
        t.tick,
        truncate(driver, 18),
        p.speed * 3.6,
        p.rpm,
        p.gear,
        p.throttle * 100.0,
        p.brake * 100.0,
        p.clutch * 100.0,
        p.lap,
        p.lap_dist_pct * 100.0,
        p.fuel_level.map_or("-".into(), |f| format!("{f:.1}")),
        p.delta_to_best.map_or("-".into(), |d| format!("{d:+.2}")),
        in_world,
        t.session.phase,
        flags.join(","),
    )
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn vars(input: &Input) -> Result<()> {
    let mut feed = Feed::open(input)?;
    let mut t = Telemetry::default();
    if !feed.next(&mut t, None)? {
        bail!("no telemetry received");
    }
    let src = feed.source();
    let vars = src.vars().context("no variable table")?;
    let row = src.raw_row();
    println!("{} variables", vars.len());
    for v in vars.iter() {
        let value = match VarTable::value(v, &row, 0) {
            Some(x) if v.count == 1 => x.to_string(),
            Some(x) => format!("{x} (+{} more)", v.count - 1),
            None => "?".into(),
        };
        println!(
            "{:<32} {:<8} {:>3} {:<10} {:<24} {}",
            v.name,
            v.ty.name(),
            v.count,
            v.unit,
            value,
            v.desc
        );
    }
    if !vars.rejected.is_empty() {
        println!(
            "rejected (out of bounds or unknown type): {:?}",
            vars.rejected
        );
    }
    Ok(())
}

fn session(input: &Input, raw: bool) -> Result<()> {
    let mut feed = Feed::open(input)?;
    let mut t = Telemetry::default();
    while feed.source().session().is_none() {
        if !feed.next(&mut t, None)? {
            bail!(
                "no session info received{}",
                feed.source()
                    .last_error()
                    .map_or(String::new(), |e| format!(" ({e})"))
            );
        }
    }
    let src = feed.source();
    if raw {
        print!("{}", src.session_yaml().unwrap_or_default());
    } else {
        println!("{:#?}", src.session().context("no session")?);
    }
    Ok(())
}

/// Running min/max of one value.
#[derive(Default)]
struct Range(Option<(f64, f64)>);

impl Range {
    fn add(&mut self, v: f64) {
        self.0 = Some(match self.0 {
            None => (v, v),
            Some((lo, hi)) => (lo.min(v), hi.max(v)),
        });
    }
}

impl std::fmt::Display for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => write!(f, "-"),
            Some((lo, hi)) => write!(f, "{lo:.2} .. {hi:.2}"),
        }
    }
}

fn check(path: &std::path::Path) -> Result<()> {
    let wall = Instant::now();
    let mut tape = IrdtTape::open(path)?;
    let mut src = tape_source(tape.memory());
    let mut t = Telemetry::default();

    let (mut frames, mut sessions, mut gaps, mut disconnects) = (0u64, 0u64, 0u64, 0u64);
    let (mut updated, mut not_updated, mut tick_jumps) = (0u64, 0u64, 0u64);
    let mut poll_ns: Vec<u32> = Vec::with_capacity(tape.record_count as usize);
    let mut revision_tick_ns: Vec<u32> = Vec::new();
    let mut copy_ns: Vec<u32> = Vec::new();
    let mut hand_off_ns: Vec<u32> = Vec::new();
    let mut parse_ns: Vec<u32> = Vec::new();
    let mut parse_timeouts = 0u64;
    let mut last_tick: Option<u64> = None;
    let mut elapsed = 0.0f64;

    let (mut speed, mut rpm, mut thr, mut brk, mut clu, mut steer) = Default::default();
    let (mut gear, mut lap, mut fuel, mut delta) = (
        Range::default(),
        Range::default(),
        Range::default(),
        Range::default(),
    );
    let mut max_in_world = 0usize;
    let mut flags_seen = BTreeSet::new();
    let mut phases_seen = BTreeSet::new();
    let mut player_idx = BTreeSet::new();

    while let Some(record) = tape.next_record()? {
        match record {
            Record::Frame { elapsed_s } => {
                frames += 1;
                elapsed = elapsed_s;
                let revisions = src.session_stats().revisions;
                let begin = Instant::now();
                let poll = src.poll(Duration::ZERO, &mut t);
                let ns = as_ns(begin.elapsed());
                let handed_off = src.session_stats().revisions > revisions;
                match poll {
                    Poll::Updated { .. } if handed_off => {
                        updated += 1;
                        revision_tick_ns.push(ns);
                        let stats = src.session_stats();
                        copy_ns.extend(stats.last_copy.map(as_ns));
                        hand_off_ns.extend(stats.last_hand_off.map(as_ns));
                        // Untimed: let the worker finish so the replay stays
                        // deterministic and every revision gets parsed.
                        if src.wait_for_session(Duration::from_secs(5)) {
                            parse_ns.extend(src.session_stats().last_parse.map(as_ns));
                        } else {
                            parse_timeouts += 1;
                        }
                    }
                    Poll::Updated { .. } => {
                        updated += 1;
                        poll_ns.push(ns);
                    }
                    _ => not_updated += 1,
                }
                if last_tick.is_some_and(|prev| t.tick != prev + 1) {
                    tick_jumps += 1;
                }
                last_tick = Some(t.tick);

                let p = &t.player;
                add_range(&mut speed, f64::from(p.speed) * 3.6);
                add_range(&mut rpm, f64::from(p.rpm));
                add_range(&mut thr, f64::from(p.throttle));
                add_range(&mut brk, f64::from(p.brake));
                add_range(&mut clu, f64::from(p.clutch));
                add_range(&mut steer, f64::from(p.steering).to_degrees());
                gear.add(f64::from(p.gear));
                lap.add(f64::from(p.lap));
                if let Some(f) = p.fuel_level {
                    fuel.add(f64::from(f));
                }
                if let Some(d) = p.delta_to_best {
                    delta.add(f64::from(d));
                }
                max_in_world =
                    max_in_world.max(t.cars.iter().filter(|c| c.surface.is_in_world()).count());
                flags_seen.extend(t.session.flags.names());
                phases_seen.insert(format!("{:?}", t.session.phase));
                if let Some(i) = p.car_idx {
                    player_idx.insert(i);
                }
            }
            Record::SessionInfo => sessions += 1,
            Record::Gap => gaps += 1,
            Record::Disconnect => disconnects += 1,
            Record::End => break,
        }
    }

    println!("tape            {}", path.display());
    println!(
        "duration        {elapsed:.1} s of telemetry, checked in {:.2} s",
        wall.elapsed().as_secs_f64()
    );
    println!(
        "records         {frames} frames, {sessions} session revisions, {gaps} gaps, {disconnects} disconnects"
    );
    println!(
        "adapter         {updated} ticks delivered, {not_updated} frames not delivered, {tick_jumps} tick discontinuities"
    );
    let stats = src.session_stats();
    println!(
        "session parses  {} applied of {} revisions{}{}",
        stats.applied,
        stats.revisions,
        if parse_timeouts > 0 {
            format!(", {parse_timeouts} timed out")
        } else {
            String::new()
        },
        src.last_error()
            .map_or(String::new(), |e| format!(" (last error: {e})"))
    );
    println!("tick cost       {}", percentiles(&mut poll_ns));
    println!("tick + revision {}", percentiles(&mut revision_tick_ns));
    println!("  copy out      {}", percentiles(&mut copy_ns));
    println!("  hand-off      {}", percentiles(&mut hand_off_ns));
    println!("decode + parse  {}", percentiles(&mut parse_ns));

    if let Some(info) = src.session() {
        println!();
        println!(
            "track           {} ({}), {:.0} m",
            info.track.name,
            info.track.config.as_deref().unwrap_or("-"),
            info.track.length_m.unwrap_or(0.0)
        );
        let sessions: Vec<_> = info
            .sessions
            .iter()
            .map(|s| {
                format!(
                    "{} {:?} laps={:?} time={:?}",
                    s.name, s.kind, s.laps, s.duration
                )
            })
            .collect();
        println!("sessions        {}", sessions.join(" | "));
        let real = info
            .drivers
            .iter()
            .filter(|d| !d.is_pace_car && !d.is_spectator)
            .count();
        println!(
            "drivers         {} entries, {real} racing",
            info.drivers.len()
        );
        let classes: BTreeMap<_, _> = info
            .drivers
            .iter()
            .map(|d| (d.class.id, d.class.name.as_deref().unwrap_or("(unnamed)")))
            .collect();
        let classes: Vec<_> = classes
            .iter()
            .map(|(id, name)| format!("{id} {name}"))
            .collect();
        println!("classes         {}", classes.join(" | "));
        println!(
            "player car      telemetry idx {:?}, session idx {:?}, shift {:?} rpm, redline {:?}, tank {:?} L",
            player_idx,
            info.player_car_idx,
            info.player_car.shift_rpm,
            info.player_car.redline_rpm,
            info.player_car.fuel_capacity_l
        );
    }

    println!();
    println!("speed km/h      {speed}");
    println!("rpm             {rpm}");
    println!("gear            {gear}");
    println!("throttle        {thr}");
    println!("brake           {brk}");
    println!("clutch pedal    {clu}");
    println!("steering deg    {steer}");
    println!("lap             {lap}");
    println!("fuel L          {fuel}");
    println!("delta to best   {delta}");
    println!("cars in world   max {max_in_world}");
    println!("flags seen      {flags_seen:?}");
    println!("phases seen     {phases_seen:?}");
    Ok(())
}

fn add_range(r: &mut Range, v: f64) {
    r.add(v);
}

fn as_ns(d: Duration) -> u32 {
    d.as_nanos().min(u128::from(u32::MAX)) as u32
}

fn percentiles(ns: &mut [u32]) -> String {
    if ns.is_empty() {
        return "-".into();
    }
    ns.sort_unstable();
    let pick = |q: f64| f64::from(ns[((ns.len() - 1) as f64 * q) as usize]) / 1000.0;
    let mean = ns.iter().map(|&n| f64::from(n)).sum::<f64>() / ns.len() as f64 / 1000.0;
    format!(
        "n={} mean {mean:.1} µs, p50 {:.1} µs, p99 {:.1} µs, max {:.1} µs",
        ns.len(),
        pick(0.5),
        pick(0.99),
        pick(1.0)
    )
}
