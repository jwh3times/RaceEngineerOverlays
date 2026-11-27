use std::sync::{Arc, Mutex};
use std::time::Duration;

use reo_iracing::{Connection, IRacingSource};
use reo_irsdk::testing::{Image, ImageBuilder};
use reo_irsdk::{Client, SharedMemory, VarType};
use reo_model::{Flags, Poll, SessionKind, SessionPhase, Telemetry, TelemetrySource, TrackSurface};

const CARS: usize = 64;

const SESSION_YAML: &str = "\
---
WeekendInfo:
 TrackName: spa 2024 up
 TrackDisplayName: Circuit de Spa-Francorchamps
 TrackConfigName: Grand Prix Pits
 TrackLength: 6.93 km
SessionInfo:
 Sessions:
 - SessionNum: 0
   SessionLaps: unlimited
   SessionTime: 1800.0000 sec
   SessionType: Practice
   SessionName: PRACTICE
 - SessionNum: 1
   SessionLaps: 20
   SessionTime: unlimited
   SessionType: Race
   SessionName: RACE
DriverInfo:
 DriverCarIdx: 3
 DriverCarRedLine: 8500.000
 DriverCarFuelMaxLtr: 120.000
 DriverCarSLShiftRPM: 8100.000
 Drivers:
 - CarIdx: 0
   UserName: Pace Car
   AbbrevName:
   TeamName: Pace Car
   CarNumber: \"0\"
   CarScreenName: safety pcporsche911cup
   CarClassID: 11
   CarClassShortName:
   CarClassColor: 0xffffff
   IRating: 0
   LicString: R 0.00
   CarIsPaceCar: 1
   IsSpectator: 0
 - CarIdx: 3
   UserName: Jürgen [CLUB] Müller
   AbbrevName: Müller, J
   TeamName: Team: Fast & Furious
   CarNumber: \"07\"
   CarScreenName: Porsche 911 GT3 R (992)
   CarClassID: 4083
   CarClassShortName: GT3 Class
   CarClassColor: 0xffda59
   CarClassEstLapTime: 137.4512
   IRating: 2875
   LicString: A 4.99
   CarIsPaceCar: 0
   IsSpectator: 0
";

fn image() -> Image {
    builder().build()
}

fn builder() -> ImageBuilder {
    ImageBuilder::new()
        .var("SessionTime", VarType::Double, 1)
        .var("SessionNum", VarType::Int, 1)
        .var("SessionState", VarType::Int, 1)
        .var("SessionFlags", VarType::BitField, 1)
        .var("SessionTimeRemain", VarType::Double, 1)
        .var("SessionLapsRemainEx", VarType::Int, 1)
        .var("PlayerCarIdx", VarType::Int, 1)
        .var("Speed", VarType::Float, 1)
        .var("RPM", VarType::Float, 1)
        .var("Gear", VarType::Int, 1)
        .var("Throttle", VarType::Float, 1)
        .var("Brake", VarType::Float, 1)
        .var("Clutch", VarType::Float, 1)
        .var("SteeringWheelAngle", VarType::Float, 1)
        .var("FuelLevel", VarType::Float, 1)
        .var("Lap", VarType::Int, 1)
        .var("LapDistPct", VarType::Float, 1)
        .var("LapLastLapTime", VarType::Float, 1)
        .var("LapBestLapTime", VarType::Float, 1)
        .var("LapDeltaToBestLap", VarType::Float, 1)
        .var("LapDeltaToBestLap_OK", VarType::Bool, 1)
        .var("OnPitRoad", VarType::Bool, 1)
        .var("EngineWarnings", VarType::BitField, 1)
        .var("CarIdxLap", VarType::Int, CARS)
        .var("CarIdxLapDistPct", VarType::Float, CARS)
        .var("CarIdxPosition", VarType::Int, CARS)
        .var("CarIdxTrackSurface", VarType::Int, CARS)
        .var("CarIdxLastLapTime", VarType::Float, CARS)
}

/// Thread-safe image the test can edit after handing it to the source.
#[derive(Clone)]
struct Shared(Arc<Mutex<Image>>);

impl Shared {
    fn edit(&self, f: impl FnOnce(&mut Image)) {
        f(&mut self.0.lock().unwrap());
    }
}

impl SharedMemory for Shared {
    fn len(&self) -> usize {
        self.0.lock().unwrap().bytes.len()
    }
    fn read_into(&self, offset: usize, dst: &mut [u8]) -> bool {
        self.0.lock().unwrap().bytes.read_into(offset, dst)
    }
}

fn source(mem: &Shared) -> IRacingSource<Shared> {
    let mem = mem.clone();
    IRacingSource::with_opener(move || {
        Some(Connection {
            client: Client::new(mem.clone()),
            waiter: None,
        })
    })
}

fn race_tick(img: &mut Image) {
    img.set(0, "SessionTime", 0, 512.25f64);
    img.set(0, "SessionNum", 0, 1i32);
    img.set(0, "SessionState", 0, 4i32);
    img.set(
        0,
        "SessionFlags",
        0,
        0x0000_0004u32 | 0x0000_0020 | 0x0010_0000,
    );
    img.set(0, "SessionTimeRemain", 0, 604_800.0f64);
    img.set(0, "SessionLapsRemainEx", 0, 12i32);
    img.set(0, "PlayerCarIdx", 0, 3i32);
    img.set(0, "Speed", 0, 55.5f32);
    img.set(0, "RPM", 0, 7200.0f32);
    img.set(0, "Gear", 0, 4i32);
    img.set(0, "Throttle", 0, 0.8f32);
    img.set(0, "Brake", 0, 0.0f32);
    img.set(0, "Clutch", 0, 1.0f32);
    img.set(0, "FuelLevel", 0, 42.0f32);
    img.set(0, "Lap", 0, 8i32);
    img.set(0, "LapDistPct", 0, 0.5f32);
    img.set(0, "LapLastLapTime", 0, -1.0f32);
    img.set(0, "LapBestLapTime", 0, 138.2f32);
    img.set(0, "LapDeltaToBestLap", 0, -0.35f32);
    img.set(0, "LapDeltaToBestLap_OK", 0, true);
    img.set(0, "EngineWarnings", 0, 0x10u32);
    for i in 0..CARS {
        img.set(0, "CarIdxTrackSurface", i, -1i32);
    }
    img.set(0, "CarIdxTrackSurface", 3, 3i32);
    img.set(0, "CarIdxLapDistPct", 3, 0.5f32);
    img.set(0, "CarIdxPosition", 3, 2i32);
    img.set(0, "CarIdxLap", 3, 8i32);
    img.set(0, "CarIdxLastLapTime", 3, 139.9f32);
    img.set(0, "CarIdxTrackSurface", 5, 1i32);
    img.set(0, "CarIdxPosition", 5, 0i32);
    img.set_session_info(SESSION_YAML, 4);
    img.set_tick(0, 900);
}

#[test]
fn maps_a_tick_onto_the_model() {
    let mem = Shared(Arc::new(Mutex::new(image())));
    mem.edit(race_tick);
    let mut src = source(&mem);
    let mut t = Telemetry::default();

    // The session parses on the worker, so this tick reports no change yet
    // (see `keeps_the_previous_session_until_the_new_parse_lands`).
    assert_eq!(
        src.poll(Duration::ZERO, &mut t),
        Poll::Updated {
            session_changed: false
        }
    );
    assert_eq!(src.last_error(), None);

    assert_eq!(t.tick, 900);
    assert_eq!(t.session_time, 512.25);
    assert_eq!(t.session.session_num, Some(1));
    assert_eq!(t.session.phase, SessionPhase::Racing);
    assert!(
        t.session
            .flags
            .contains(Flags::GREEN | Flags::BLUE | Flags::MEATBALL)
    );
    assert!(!t.session.flags.contains(Flags::YELLOW));
    assert_eq!(t.session.time_remaining, None, "unlimited time is None");
    assert_eq!(t.session.laps_remaining, Some(12));

    let p = &t.player;
    assert_eq!(p.car_idx, Some(3));
    assert_eq!((p.speed, p.rpm, p.gear), (55.5, 7200.0, 4));
    assert_eq!(p.clutch, 0.0, "fully engaged clutch is an unpressed pedal");
    assert_eq!(p.fuel_level, Some(42.0));
    assert_eq!(p.fuel_fraction, None, "var not published");
    assert_eq!(p.lap_last_time, None, "-1 means no lap yet");
    assert_eq!(p.lap_best_time, Some(138.2));
    assert_eq!(p.delta_to_best, Some(-0.35));
    assert!(p.pit_limiter);

    assert_eq!(t.cars.len(), CARS);
    let me = t.player_car().unwrap();
    assert_eq!(me.idx, 3);
    assert_eq!(me.surface, TrackSurface::OnTrack);
    assert_eq!(me.position, Some(2));
    assert_eq!(me.last_lap_time, Some(139.9));
    assert_eq!(t.cars[5].surface, TrackSurface::InPitStall);
    assert_eq!(t.cars[5].position, None, "0 means unclassified");
    assert!(!t.cars[10].surface.is_in_world());
}

#[test]
fn parses_session_yaml_with_hostile_names() {
    let mem = Shared(Arc::new(Mutex::new(image())));
    mem.edit(race_tick);
    let mut src = source(&mem);
    src.poll(Duration::ZERO, &mut Telemetry::default());
    assert!(src.wait_for_session(WAIT));

    let info = src.session().expect("session parsed");
    assert_eq!(info.revision, 4);
    assert_eq!(info.track.name, "Circuit de Spa-Francorchamps");
    assert_eq!(info.track.config.as_deref(), Some("Grand Prix Pits"));
    assert!((info.track.length_m.unwrap() - 6930.0).abs() < 0.5);

    assert_eq!(info.sessions.len(), 2);
    assert_eq!(info.sessions[0].kind, SessionKind::Practice);
    assert_eq!(info.sessions[0].laps, None);
    assert_eq!(info.sessions[0].duration, Some(Duration::from_secs(1800)));
    assert_eq!(info.sessions[1].kind, SessionKind::Race);
    assert_eq!(info.sessions[1].laps, Some(20));
    assert_eq!(info.sessions[1].duration, None);

    assert_eq!(info.player_car_idx, Some(3));
    assert_eq!(info.player_car.shift_rpm, Some(8100.0));
    assert_eq!(info.player_car.fuel_capacity_l, Some(120.0));

    let pace = info.driver(0).unwrap();
    assert!(pace.is_pace_car);
    assert_eq!(pace.abbrev_name, None);
    assert_eq!(pace.class.color, Some(0xffffff));

    let me = info.driver(3).unwrap();
    assert_eq!(me.name, "Jürgen [CLUB] Müller");
    assert_eq!(me.team_name.as_deref(), Some("Team: Fast & Furious"));
    assert_eq!(me.car_number, "07");
    assert_eq!(me.class.color, Some(0xffda59));
    assert_eq!(me.class.est_lap_time, Some(137.4512));
    assert_eq!(me.rating, Some(2875));
    assert_eq!(me.license.as_deref(), Some("A 4.99"));
}

/// Long enough for any parse on a loaded CI runner; a test that waits this long has failed.
const WAIT: Duration = Duration::from_secs(5);

fn revision(src: &IRacingSource<Shared>) -> Option<i64> {
    src.session().map(|s| s.revision)
}

/// Publishes the next tick, optionally with a new session-info revision, and polls it.
fn next_tick(
    mem: &Shared,
    src: &mut IRacingSource<Shared>,
    tick: i32,
    session: Option<(&str, i32)>,
) -> Poll {
    mem.edit(|img| {
        if let Some((yaml, rev)) = session {
            img.set_session_info(yaml, rev);
        }
        img.set_tick(0, tick);
    });
    src.poll(Duration::ZERO, &mut Telemetry::default())
}

const UNCHANGED: Poll = Poll::Updated {
    session_changed: false,
};
const CHANGED: Poll = Poll::Updated {
    session_changed: true,
};

#[test]
fn keeps_the_previous_session_until_the_new_parse_lands() {
    let mem = Shared(Arc::new(Mutex::new(image())));
    mem.edit(race_tick);
    let mut src = source(&mem);

    // The tick that carries the first revision hands it off and returns at once.
    assert_eq!(
        src.poll(Duration::ZERO, &mut Telemetry::default()),
        UNCHANGED
    );
    assert_eq!(revision(&src), None);
    assert!(src.wait_for_session(WAIT));
    assert_eq!(revision(&src), Some(4));
    assert_eq!(next_tick(&mem, &mut src, 901, None), CHANGED);

    // A new revision: consumers keep the old session until the parse lands.
    assert_eq!(
        next_tick(&mem, &mut src, 902, Some((SESSION_YAML, 5))),
        UNCHANGED
    );
    assert_eq!(revision(&src), Some(4));
    assert!(src.wait_for_session(WAIT));
    assert_eq!(revision(&src), Some(5));
    assert_eq!(next_tick(&mem, &mut src, 903, None), CHANGED);
    assert_eq!(next_tick(&mem, &mut src, 904, None), UNCHANGED);
}

/// A source whose first session (revision 4) has been parsed and reported.
fn settled_source(mem: &Shared) -> IRacingSource<Shared> {
    mem.edit(race_tick);
    let mut src = source(mem);
    src.poll(Duration::ZERO, &mut Telemetry::default());
    assert!(src.wait_for_session(WAIT));
    assert_eq!(next_tick(mem, &mut src, 901, None), CHANGED);
    src
}

#[test]
fn polling_alone_picks_up_a_finished_parse() {
    let mem = Shared(Arc::new(Mutex::new(image())));
    let mut src = settled_source(&mem);
    next_tick(&mem, &mut src, 902, Some((SESSION_YAML, 5)));

    // No wait_for_session: a live consumer only ever polls.
    let deadline = std::time::Instant::now() + WAIT;
    let mut tick = 903;
    let mut changed = false;
    while revision(&src) != Some(5) {
        assert!(
            std::time::Instant::now() < deadline,
            "the parse never landed"
        );
        std::thread::sleep(Duration::from_millis(1));
        changed |= next_tick(&mem, &mut src, tick, None) == CHANGED;
        tick += 1;
    }
    assert!(changed, "the tick that applied it reported the change");
}

/// How the worker collapses a burst depends on thread timing, so this checks
/// only the outcome; the collapsing itself is tested beside the worker.
#[test]
fn a_burst_of_revisions_ends_on_the_newest() {
    let mem = Shared(Arc::new(Mutex::new(image())));
    let mut src = settled_source(&mem);

    let mut seen = vec![revision(&src)];
    for (tick, rev) in [(902, 5), (903, 6), (904, 7)] {
        next_tick(&mem, &mut src, tick, Some((SESSION_YAML, rev)));
        seen.push(revision(&src));
    }
    assert!(src.wait_for_session(WAIT));
    for tick in 905..910 {
        next_tick(&mem, &mut src, tick, None);
        seen.push(revision(&src));
    }

    assert!(seen.is_sorted(), "a revision went backwards: {seen:?}");
    assert_eq!(seen.last(), Some(&Some(7)));
    let stats = src.session_stats();
    assert_eq!(stats.revisions, 4);
    assert!((2..=4).contains(&stats.applied), "{stats:?}");
}

#[test]
fn a_parse_error_keeps_the_previous_session() {
    let mem = Shared(Arc::new(Mutex::new(image())));
    let mut src = settled_source(&mem);

    // A mapping followed by a sequence at the same level is not YAML.
    let broken = "WeekendInfo:\n TrackName: spa\n- stray\n";
    next_tick(&mem, &mut src, 902, Some((broken, 5)));
    assert_eq!(src.last_error(), None, "the parse runs on the worker");
    assert!(src.wait_for_session(WAIT));

    assert_eq!(revision(&src), Some(4));
    let error = src.last_error().expect("parse error reported");
    assert!(error.starts_with("session info YAML"), "{error}");
    assert_eq!(src.session_yaml(), Some(broken), "the newest raw YAML");
    assert_eq!(next_tick(&mem, &mut src, 903, None), UNCHANGED);
}

/// Guards against a shutdown that hangs: the drop must close the worker's
/// channels before joining it. A large session keeps the parse busy.
#[test]
fn dropping_the_source_with_parses_pending_returns_promptly() {
    let mut yaml = String::from(SESSION_YAML);
    for idx in 100..1500 {
        yaml.push_str(&format!(
            " - CarIdx: {idx}\n   UserName: Driver {idx}\n   CarScreenName: Car\n   CarClassID: 1\n   CarClassShortName: \n"
        ));
    }
    let mut img = builder().session_capacity(yaml.len() + 1).build();
    race_tick(&mut img);
    img.set_session_info(&yaml, 4);
    let mem = Shared(Arc::new(Mutex::new(img)));
    let mut src = source(&mem);
    src.poll(Duration::ZERO, &mut Telemetry::default());
    next_tick(&mem, &mut src, 901, Some((&yaml, 5)));
    assert_eq!(src.session_stats().revisions, 2);

    let (done, dropped) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        drop(src);
        done.send(()).unwrap();
    });
    dropped.recv_timeout(WAIT).expect("drop returned");
}

#[test]
fn reports_disconnected_then_recovers() {
    let mem = Shared(Arc::new(Mutex::new(image())));
    mem.edit(|img| {
        race_tick(img);
        img.set_connected(false);
    });
    let mut src = source(&mem);
    let mut t = Telemetry::default();
    assert_eq!(src.poll(Duration::ZERO, &mut t), Poll::Disconnected);

    mem.edit(|img| img.set_connected(true));
    assert!(matches!(
        src.poll(Duration::ZERO, &mut t),
        Poll::Updated { .. }
    ));
    assert_eq!(src.poll(Duration::ZERO, &mut t), Poll::Unchanged);
}

#[test]
fn keeps_running_when_the_sim_is_not_open() {
    let mut src: IRacingSource<Shared> = IRacingSource::with_opener(|| None);
    let mut t = Telemetry::default();
    assert_eq!(
        src.poll(Duration::from_millis(1), &mut t),
        Poll::Disconnected
    );
    assert_eq!(
        src.poll(Duration::from_millis(1), &mut t),
        Poll::Disconnected
    );
    assert!(src.session().is_none());
}
