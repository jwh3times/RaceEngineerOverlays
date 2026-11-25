//! Maps one iRacing telemetry row onto [`Telemetry`].

use reo_irsdk::{Var, VarTable, VarValue};
use reo_model::{CarTelemetry, Flags, SessionPhase, Telemetry, TrackSurface};

/// iRacing's marker for "no lap limit".
const UNLIMITED_LAPS: i32 = 32_767;
/// iRacing's marker for "no time limit" (one week), with float slack.
const UNLIMITED_TIME_S: f64 = 604_000.0;
const ENGINE_WARNING_PIT_LIMITER: u32 = 0x10;

/// Typed handles for every variable the adapter reads, resolved once per connection.
/// A variable the sim doesn't publish stays `None` and its model field keeps its default.
#[derive(Clone, Debug, Default)]
pub struct Handles {
    session_time: Option<Var<f64>>,
    session_num: Option<Var<i32>>,
    session_state: Option<Var<i32>>,
    session_flags: Option<Var<u32>>,
    session_time_remain: Option<Var<f64>>,
    session_laps_remain: Option<Var<i32>>,

    player_car_idx: Option<Var<i32>>,
    speed: Option<Var<f32>>,
    rpm: Option<Var<f32>>,
    gear: Option<Var<i32>>,
    throttle: Option<Var<f32>>,
    brake: Option<Var<f32>>,
    clutch: Option<Var<f32>>,
    steering: Option<Var<f32>>,
    fuel_level: Option<Var<f32>>,
    fuel_pct: Option<Var<f32>>,
    lap: Option<Var<i32>>,
    lap_dist_pct: Option<Var<f32>>,
    lap_current: Option<Var<f32>>,
    lap_last: Option<Var<f32>>,
    lap_best: Option<Var<f32>>,
    delta_best: Option<Var<f32>>,
    delta_best_ok: Option<Var<bool>>,
    on_pit_road: Option<Var<bool>>,
    engine_warnings: Option<Var<u32>>,

    car_lap: Option<Var<i32>>,
    car_lap_completed: Option<Var<i32>>,
    car_lap_dist_pct: Option<Var<f32>>,
    car_position: Option<Var<i32>>,
    car_class_position: Option<Var<i32>>,
    car_on_pit_road: Option<Var<bool>>,
    car_surface: Option<Var<i32>>,
    car_est_time: Option<Var<f32>>,
    car_last_lap: Option<Var<f32>>,
    car_best_lap: Option<Var<f32>>,
}

impl Handles {
    pub fn resolve(vars: &VarTable) -> Self {
        Self {
            session_time: vars.handle("SessionTime"),
            session_num: vars.handle("SessionNum"),
            session_state: vars.handle("SessionState"),
            session_flags: vars.handle("SessionFlags"),
            session_time_remain: vars.handle("SessionTimeRemain"),
            session_laps_remain: vars.handle("SessionLapsRemainEx"),

            player_car_idx: vars.handle("PlayerCarIdx"),
            speed: vars.handle("Speed"),
            rpm: vars.handle("RPM"),
            gear: vars.handle("Gear"),
            throttle: vars.handle("Throttle"),
            brake: vars.handle("Brake"),
            clutch: vars.handle("Clutch"),
            steering: vars.handle("SteeringWheelAngle"),
            fuel_level: vars.handle("FuelLevel"),
            fuel_pct: vars.handle("FuelLevelPct"),
            lap: vars.handle("Lap"),
            lap_dist_pct: vars.handle("LapDistPct"),
            lap_current: vars.handle("LapCurrentLapTime"),
            lap_last: vars.handle("LapLastLapTime"),
            lap_best: vars.handle("LapBestLapTime"),
            delta_best: vars.handle("LapDeltaToBestLap"),
            delta_best_ok: vars.handle("LapDeltaToBestLap_OK"),
            on_pit_road: vars.handle("OnPitRoad"),
            engine_warnings: vars.handle("EngineWarnings"),

            car_lap: vars.handle("CarIdxLap"),
            car_lap_completed: vars.handle("CarIdxLapCompleted"),
            car_lap_dist_pct: vars.handle("CarIdxLapDistPct"),
            car_position: vars.handle("CarIdxPosition"),
            car_class_position: vars.handle("CarIdxClassPosition"),
            car_on_pit_road: vars.handle("CarIdxOnPitRoad"),
            car_surface: vars.handle("CarIdxTrackSurface"),
            car_est_time: vars.handle("CarIdxEstTime"),
            car_last_lap: vars.handle("CarIdxLastLapTime"),
            car_best_lap: vars.handle("CarIdxBestLapTime"),
        }
    }

    /// Writes `row` into `out`, reusing `out.cars`' allocation.
    pub fn fill(&self, row: &[u8], tick: i32, out: &mut Telemetry) {
        out.tick = u64::try_from(tick).unwrap_or(0);
        out.session_time = get(&self.session_time, row).unwrap_or(0.0);

        let s = &mut out.session;
        s.session_num = get(&self.session_num, row).and_then(|n| u16::try_from(n).ok());
        s.phase = phase(get(&self.session_state, row).unwrap_or(0));
        s.flags = flags(get(&self.session_flags, row).unwrap_or(0));
        s.time_remaining =
            get(&self.session_time_remain, row).filter(|&t| (0.0..UNLIMITED_TIME_S).contains(&t));
        s.laps_remaining =
            get(&self.session_laps_remain, row).filter(|&l| (0..UNLIMITED_LAPS).contains(&l));

        let p = &mut out.player;
        p.car_idx = get(&self.player_car_idx, row).and_then(|i| u16::try_from(i).ok());
        p.speed = get(&self.speed, row).unwrap_or(0.0);
        p.rpm = get(&self.rpm, row).unwrap_or(0.0);
        p.gear = get(&self.gear, row).map_or(0, |g| g.clamp(-1, 127) as i8);
        p.throttle = get(&self.throttle, row).unwrap_or(0.0);
        p.brake = get(&self.brake, row).unwrap_or(0.0);
        // iRacing reports clutch *engagement* (1 = pedal released); the model wants pedal travel.
        p.clutch = get(&self.clutch, row).map_or(0.0, |c| (1.0 - c).clamp(0.0, 1.0));
        p.steering = get(&self.steering, row).unwrap_or(0.0);
        p.fuel_level = get(&self.fuel_level, row);
        p.fuel_fraction = get(&self.fuel_pct, row);
        p.lap = get(&self.lap, row).unwrap_or(0);
        p.lap_dist_pct = get(&self.lap_dist_pct, row).unwrap_or(0.0);
        p.lap_current_time = get(&self.lap_current, row).filter(|&t| t >= 0.0);
        p.lap_last_time = lap_time(get(&self.lap_last, row));
        p.lap_best_time = lap_time(get(&self.lap_best, row));
        p.delta_to_best = get(&self.delta_best_ok, row)
            .unwrap_or(false)
            .then(|| get(&self.delta_best, row))
            .flatten();
        p.on_pit_road = get(&self.on_pit_road, row).unwrap_or(false);
        p.pit_limiter =
            get(&self.engine_warnings, row).is_some_and(|w| w & ENGINE_WARNING_PIT_LIMITER != 0);

        let slots = self.car_lap_dist_pct.map_or(0, |v| v.count());
        out.cars.resize_with(slots, CarTelemetry::default);
        for (i, car) in out.cars.iter_mut().enumerate() {
            car.idx = i as u16;
            car.lap = at(&self.car_lap, row, i).unwrap_or(0);
            car.laps_completed = at(&self.car_lap_completed, row, i).unwrap_or(0);
            car.lap_dist_pct = at(&self.car_lap_dist_pct, row, i).unwrap_or(-1.0);
            car.position = at(&self.car_position, row, i).and_then(position);
            car.class_position = at(&self.car_class_position, row, i).and_then(position);
            car.on_pit_road = at(&self.car_on_pit_road, row, i).unwrap_or(false);
            car.surface = surface(at(&self.car_surface, row, i).unwrap_or(-1));
            car.est_time = at(&self.car_est_time, row, i).filter(|&t| t > 0.0);
            car.last_lap_time = lap_time(at(&self.car_last_lap, row, i));
            car.best_lap_time = lap_time(at(&self.car_best_lap, row, i));
        }
    }
}

fn get<T: VarValue>(var: &Option<Var<T>>, row: &[u8]) -> Option<T> {
    var.as_ref()?.get(row)
}

fn at<T: VarValue>(var: &Option<Var<T>>, row: &[u8], index: usize) -> Option<T> {
    var.as_ref()?.at(row, index)
}

/// iRacing uses -1 or 0 for "no lap time yet".
fn lap_time(t: Option<f32>) -> Option<f32> {
    t.filter(|&t| t > 0.0)
}

/// iRacing uses 0 for "no position yet".
fn position(p: i32) -> Option<u16> {
    u16::try_from(p).ok().filter(|&p| p > 0)
}

fn surface(raw: i32) -> TrackSurface {
    match raw {
        0 => TrackSurface::OffTrack,
        1 => TrackSurface::InPitStall,
        2 => TrackSurface::ApproachingPits,
        3 => TrackSurface::OnTrack,
        _ => TrackSurface::NotInWorld,
    }
}

fn phase(raw: i32) -> SessionPhase {
    match raw {
        1 => SessionPhase::Gridding,
        2 => SessionPhase::Warmup,
        3 => SessionPhase::ParadeLaps,
        4 => SessionPhase::Racing,
        5 => SessionPhase::Checkered,
        6 => SessionPhase::CoolDown,
        _ => SessionPhase::Unknown,
    }
}

/// Maps `irsdk_Flags` bits onto the model's flags.
pub fn flags(raw: u32) -> Flags {
    const MAP: [(u32, Flags); 18] = [
        (0x0000_0001, Flags::CHECKERED),
        (0x0000_0002, Flags::WHITE),
        (0x0000_0004, Flags::GREEN),
        (0x0000_0008, Flags::YELLOW),
        (0x0000_0010, Flags::RED),
        (0x0000_0020, Flags::BLUE),
        (0x0000_0040, Flags::DEBRIS),
        (0x0000_0100, Flags::YELLOW_WAVING),
        (0x0000_0200, Flags::ONE_LAP_TO_GREEN),
        (0x0000_4000, Flags::CAUTION),
        (0x0000_8000, Flags::CAUTION),
        (0x0001_0000, Flags::BLACK),
        (0x0002_0000, Flags::DISQUALIFIED),
        (0x0010_0000, Flags::MEATBALL),
        (0x2000_0000, Flags::START_READY),
        (0x4000_0000, Flags::START_SET),
        (0x8000_0000, Flags::START_GO),
        (0x0000_0400, Flags::GREEN),
    ];
    let mut out = Flags::empty();
    for (bit, flag) in MAP {
        if raw & bit != 0 {
            out.insert(flag);
        }
    }
    out
}
