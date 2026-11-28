//! PROTOTYPE: a crude Relative, computed once per tick on the feed thread.

use std::sync::Arc;
use std::time::Instant;

use reo_model::{SessionInfo, Telemetry};

pub const AHEAD: usize = 3;
pub const BEHIND: usize = 3;
pub const ROWS: usize = AHEAD + 1 + BEHIND;

#[derive(Clone, Copy, Debug, Default)]
pub struct Row {
    pub car_idx: u16,
    pub position: u16,
    /// Seconds to this car on track: positive ahead, negative behind.
    pub gap: f32,
    pub is_player: bool,
    /// +1 a lap or more ahead on the road, -1 behind, 0 same lap.
    pub lap_diff: i8,
}

/// What the feed thread publishes each tick.
#[derive(Clone)]
pub struct Snap {
    pub seq: u64,
    pub published: Instant,
    pub rows: [Row; ROWS],
    pub count: usize,
    pub session: Option<Arc<SessionInfo>>,
}

impl Default for Snap {
    fn default() -> Self {
        Self {
            seq: 0,
            published: Instant::now(),
            rows: [Row::default(); ROWS],
            count: 0,
            session: None,
        }
    }
}

/// Fills `out` from one tick. `scratch` is reused so the tick allocates nothing.
pub fn compute(t: &Telemetry, session: Option<&SessionInfo>, out: &mut Snap, scratch: &mut Vec<(f32, Row)>) {
    out.count = 0;
    let Some(me) = t.player_car() else { return };
    let Some(my_est) = me.est_time else { return };
    let lap_time = session
        .and_then(|s| s.driver(me.idx))
        .and_then(|d| d.class.est_lap_time)
        .or(me.best_lap_time)
        .filter(|l| *l > 1.0)
        .unwrap_or(90.0);
    let my_dist = me.lap as f32 + me.lap_dist_pct;

    scratch.clear();
    for c in &t.cars {
        if c.idx == me.idx || !c.surface.is_in_world() {
            continue;
        }
        let Some(est) = c.est_time else { continue };
        let mut d = est - my_est;
        if d > lap_time / 2.0 {
            d -= lap_time;
        } else if d < -lap_time / 2.0 {
            d += lap_time;
        }
        let road = c.lap as f32 + c.lap_dist_pct - my_dist;
        let lap_diff = if road > 0.5 { 1 } else if road < -0.5 { -1 } else { 0 };
        scratch.push((
            d,
            Row {
                car_idx: c.idx,
                position: c.position.unwrap_or(0),
                gap: d,
                is_player: false,
                lap_diff,
            },
        ));
    }
    scratch.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));

    // Ahead: the AHEAD smallest positive gaps, farthest first.
    let first_behind = scratch.iter().position(|(d, _)| *d <= 0.0).unwrap_or(scratch.len());
    let ahead_from = first_behind.saturating_sub(AHEAD);
    for (_, r) in &scratch[ahead_from..first_behind] {
        out.rows[out.count] = *r;
        out.count += 1;
    }
    out.rows[out.count] = Row {
        car_idx: me.idx,
        position: me.position.unwrap_or(0),
        gap: 0.0,
        is_player: true,
        lap_diff: 0,
    };
    out.count += 1;
    for (_, r) in scratch[first_behind..].iter().take(BEHIND) {
        out.rows[out.count] = *r;
        out.count += 1;
    }
}
