//! Sim-agnostic model that every sim adapter fills and every overlay reads.
//!
//! Units are SI unless a name says otherwise: metres, seconds, litres, radians,
//! metres per second. Pedals and fractions are `0.0..=1.0`. A value a sim does
//! not provide is `None`, never a sentinel like `-1`.
//!
//! [`Telemetry`] changes every tick (typically 60 Hz) and is reused between
//! ticks to avoid allocation. [`SessionInfo`] changes rarely and is shared as
//! an `Arc`.

use std::sync::Arc;
use std::time::Duration;

mod flags;
pub use flags::Flags;

/// Which sim produced the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SimId {
    IRacing,
    LeMansUltimate,
    AssettoCorsaCompetizione,
    AutomobilistaTwo,
}

impl SimId {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::IRacing => "iRacing",
            Self::LeMansUltimate => "Le Mans Ultimate",
            Self::AssettoCorsaCompetizione => "Assetto Corsa Competizione",
            Self::AutomobilistaTwo => "Automobilista 2",
        }
    }
}

/// Per-tick telemetry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Telemetry {
    /// Monotonic counter from the sim; increases by one per published tick.
    pub tick: u64,
    /// Seconds since the session started.
    pub session_time: f64,
    pub session: SessionState,
    pub player: PlayerTelemetry,
    /// Every car slot the sim publishes, indexed by [`CarTelemetry::idx`].
    /// Slots without a car have `surface == TrackSurface::NotInWorld`.
    pub cars: Vec<CarTelemetry>,
}

impl Telemetry {
    pub fn player_car(&self) -> Option<&CarTelemetry> {
        self.player
            .car_idx
            .and_then(|i| self.cars.get(usize::from(i)))
    }
}

/// Live state of the current session.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionState {
    /// Index into [`SessionInfo::sessions`].
    pub session_num: Option<u16>,
    pub phase: SessionPhase,
    pub flags: Flags,
    pub time_remaining: Option<f64>,
    pub laps_remaining: Option<i32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionPhase {
    #[default]
    Unknown,
    Gridding,
    Warmup,
    ParadeLaps,
    Racing,
    Checkered,
    CoolDown,
}

/// High-rate data for the car the player is driving (or spectating, if the
/// sim exposes it that way).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerTelemetry {
    pub car_idx: Option<u16>,
    pub speed: f32,
    pub rpm: f32,
    /// -1 reverse, 0 neutral, 1.. forward gears.
    pub gear: i8,
    pub throttle: f32,
    pub brake: f32,
    /// Clutch pedal position: 0 released, 1 fully pressed.
    pub clutch: f32,
    /// Steering wheel angle in radians, positive is left.
    pub steering: f32,
    pub fuel_level: Option<f32>,
    pub fuel_fraction: Option<f32>,
    pub lap: i32,
    /// Fraction of the lap completed, `0.0..1.0`.
    pub lap_dist_pct: f32,
    pub lap_current_time: Option<f32>,
    pub lap_last_time: Option<f32>,
    pub lap_best_time: Option<f32>,
    /// Seconds ahead (negative) or behind (positive) the best lap at this point.
    pub delta_to_best: Option<f32>,
    pub on_pit_road: bool,
    pub pit_limiter: bool,
}

/// Per-car data published for every car in the session.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CarTelemetry {
    pub idx: u16,
    /// Lap the car is currently on.
    pub lap: i32,
    pub laps_completed: i32,
    pub lap_dist_pct: f32,
    pub position: Option<u16>,
    pub class_position: Option<u16>,
    pub on_pit_road: bool,
    pub surface: TrackSurface,
    /// Estimated seconds for this car's class to reach its current track position from the line.
    pub est_time: Option<f32>,
    pub last_lap_time: Option<f32>,
    pub best_lap_time: Option<f32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackSurface {
    #[default]
    NotInWorld,
    OffTrack,
    InPitStall,
    ApproachingPits,
    OnTrack,
}

impl TrackSurface {
    pub fn is_in_world(self) -> bool {
        self != Self::NotInWorld
    }
}

/// Slow-changing description of the event, drivers and cars.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionInfo {
    pub sim: Option<SimId>,
    /// Sim-specific revision; bumps whenever this struct is rebuilt.
    pub revision: i64,
    pub track: TrackInfo,
    pub sessions: Vec<SessionDefinition>,
    pub player_car_idx: Option<u16>,
    pub player_car: PlayerCarSpec,
    pub drivers: Vec<DriverEntry>,
}

impl SessionInfo {
    pub fn driver(&self, car_idx: u16) -> Option<&DriverEntry> {
        self.drivers.iter().find(|d| d.car_idx == car_idx)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackInfo {
    pub name: String,
    pub config: Option<String>,
    pub length_m: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionDefinition {
    pub num: u16,
    pub kind: SessionKind,
    /// Display name the sim uses, e.g. "Lone Qualify".
    pub name: String,
    pub laps: Option<u32>,
    pub duration: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionKind {
    #[default]
    Other,
    Practice,
    Qualifying,
    Warmup,
    Race,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerCarSpec {
    pub redline_rpm: Option<f32>,
    pub shift_rpm: Option<f32>,
    pub fuel_capacity_l: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DriverEntry {
    pub car_idx: u16,
    pub name: String,
    pub abbrev_name: Option<String>,
    pub team_name: Option<String>,
    pub car_number: String,
    pub car_name: String,
    pub class: CarClass,
    pub rating: Option<i32>,
    pub license: Option<String>,
    pub is_spectator: bool,
    pub is_pace_car: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CarClass {
    pub id: i32,
    /// `None` when the sim names no class and none can be inferred (a class of
    /// several car models); group by `id`, which is always present.
    pub name: Option<String>,
    /// 0xRRGGBB.
    pub color: Option<u32>,
    pub est_lap_time: Option<f32>,
}

/// Result of polling a [`TelemetrySource`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poll {
    /// The sim isn't running or isn't in a session.
    Disconnected,
    /// No new tick arrived before the timeout.
    Unchanged,
    /// The telemetry buffer holds a new tick.
    Updated { session_changed: bool },
}

/// A live (or replayed) stream of data from one sim.
pub trait TelemetrySource: Send {
    fn sim(&self) -> SimId;

    /// Waits up to `timeout` for a new tick and writes it into `out`.
    /// Handles connecting and reconnecting internally.
    fn poll(&mut self, timeout: Duration, out: &mut Telemetry) -> Poll;

    /// Latest session description, if one has been received.
    fn session(&self) -> Option<Arc<SessionInfo>>;
}
