//! AGENTS.md rule 2: no allocation per tick on hot paths. This binary counts
//! heap allocations made by the polling thread, so it has its own global allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reo_iracing::{Connection, IRacingSource};
use reo_irsdk::testing::{Image, ImageBuilder};
use reo_irsdk::{Client, SharedMemory, VarType};
use reo_model::{Poll, Telemetry, TelemetrySource};

struct CountingAlloc;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

fn count() {
    // `try_with`: the allocator also runs while thread-locals are torn down.
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

// SAFETY: every method forwards to `System` with the caller's arguments, so
// `System` upholds the `GlobalAlloc` contract; counting touches no heap memory.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the caller's guarantees for `alloc` pass straight through.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the caller's guarantees for `alloc_zeroed` pass straight through.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: `ptr` came from `System` (every allocation here does), and
        // the caller's guarantees for `realloc` pass straight through.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System` with this `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

/// Heap allocations `f` makes on this thread.
fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, u64) {
    let before = ALLOCATIONS.with(Cell::get);
    let out = f();
    (out, ALLOCATIONS.with(Cell::get) - before)
}

#[derive(Clone)]
struct Shared(Arc<Mutex<Image>>);

impl SharedMemory for Shared {
    fn len(&self) -> usize {
        self.0.lock().unwrap().bytes.len()
    }
    fn read_into(&self, offset: usize, dst: &mut [u8]) -> bool {
        self.0.lock().unwrap().bytes.read_into(offset, dst)
    }
}

const YAML: &str = "\
WeekendInfo:
 TrackDisplayName: Circuit de Spa-Francorchamps
DriverInfo:
 DriverCarIdx: 0
 Drivers:
 - CarIdx: 0
   UserName: Jürgen Müller
   CarNumber: \"7\"
";

const WAIT: Duration = Duration::from_secs(5);

/// A source over an image the test publishes ticks and revisions into.
struct Rig {
    mem: Shared,
    src: IRacingSource<Shared>,
    t: Telemetry,
    tick: i32,
}

impl Rig {
    /// Connected, with revisions 1 to 3 of `YAML` handed off and applied, so
    /// the copy buffer is already going round.
    fn warm() -> Self {
        let mut img = ImageBuilder::new()
            .var("SessionTime", VarType::Double, 1)
            .var("Speed", VarType::Float, 1)
            .var("CarIdxLapDistPct", VarType::Float, 64)
            .session_capacity(1 << 20)
            .build();
        img.set_session_info(YAML, 1);
        img.set_tick(0, 1);
        let mem = Shared(Arc::new(Mutex::new(img)));
        let opener_mem = mem.clone();
        let src = IRacingSource::with_opener(move || {
            Some(Connection {
                client: Client::new(opener_mem.clone()),
                waiter: None,
            })
        });
        let mut rig = Self {
            mem,
            src,
            t: Telemetry::default(),
            tick: 1,
        };
        rig.poll();
        assert!(rig.src.wait_for_session(WAIT));
        for rev in [2, 3] {
            rig.publish(Some((YAML, rev)));
            rig.poll();
            assert!(rig.src.wait_for_session(WAIT));
        }
        rig.publish(None);
        rig.poll();
        rig
    }

    /// Publishes the next tick, optionally with a new session revision.
    fn publish(&mut self, session: Option<(&str, i32)>) {
        self.tick += 1;
        let mut img = self.mem.0.lock().unwrap();
        if let Some((yaml, rev)) = session {
            img.set_session_info(yaml, rev);
        }
        img.set_tick(0, self.tick);
    }

    fn poll(&mut self) -> Poll {
        self.src.poll(Duration::ZERO, &mut self.t)
    }

    /// Polls once, returning the heap allocations the poll made.
    fn counted_poll(&mut self) -> u64 {
        let (poll, allocs) = allocations_during(|| self.src.poll(Duration::ZERO, &mut self.t));
        assert!(matches!(poll, Poll::Updated { .. }), "{poll:?}");
        allocs
    }

    fn revision(&self) -> Option<i64> {
        self.src.session().map(|s| s.revision)
    }
}

/// The polling thread's whole share of a session revision, once the source is
/// warm: the tick that hands the revision off, and every tick until its parse
/// is applied, allocate nothing.
#[test]
fn steady_state_session_revisions_cost_the_poll_thread_no_allocations() {
    let mut rig = Rig::warm();
    for rev in 4..=8 {
        rig.publish(Some((YAML, rev)));
        assert_eq!(
            rig.counted_poll(),
            0,
            "the tick that hands off revision {rev} allocated"
        );

        let deadline = Instant::now() + WAIT;
        while rig.revision() != Some(i64::from(rev)) {
            assert!(
                Instant::now() < deadline,
                "revision {rev} was never applied"
            );
            std::thread::sleep(Duration::from_millis(1));
            rig.publish(None);
            assert_eq!(
                rig.counted_poll(),
                0,
                "a tick waiting for revision {rev} allocated"
            );
        }
    }
    assert_eq!(rig.src.last_error(), None);
    assert!(
        rig.src
            .session_yaml()
            .is_some_and(|y| y.contains("Jürgen Müller"))
    );
}

/// A revision that cannot be read keeps the copy buffer for the next one.
#[test]
fn a_failed_session_read_keeps_the_copy_buffer() {
    let mut rig = Rig::warm();
    rig.publish(Some((YAML, 4)));
    {
        // A session length past any sane bound makes the read fail.
        let mut img = rig.mem.0.lock().unwrap();
        img.bytes[16..20].copy_from_slice(&i32::MAX.to_le_bytes());
    }
    rig.poll();
    assert!(rig.src.last_error().is_some(), "the read failed");

    rig.publish(Some((YAML, 5)));
    assert_eq!(rig.counted_poll(), 0, "the next hand-off allocated");
    assert!(rig.src.wait_for_session(WAIT));
    assert_eq!(rig.revision(), Some(5));
}

/// Revisions that arrive while the worker is busy replace one another; the
/// buffer of a replaced one is kept for the next hand-off.
#[test]
fn a_displaced_revision_keeps_its_copy_buffer() {
    let mut rig = Rig::warm();
    // A large session keeps the worker parsing while the next revisions arrive.
    let mut big = String::from(YAML);
    for idx in 1..3000 {
        big.push_str(&format!(
            " - CarIdx: {idx}\n   UserName: Driver {idx}\n   CarNumber: \"{idx}\"\n"
        ));
    }
    rig.publish(Some((&big, 4)));
    rig.poll();
    // The worker is parsing 4: 5 waits for it, and 6 displaces 5.
    for rev in [5, 6] {
        rig.publish(Some((&big, rev)));
        rig.poll();
    }

    rig.publish(Some((&big, 7)));
    assert_eq!(
        rig.counted_poll(),
        0,
        "the hand-off after a displaced revision allocated"
    );
    assert!(rig.src.wait_for_session(WAIT));
    assert_eq!(rig.revision(), Some(7));
    // Without a displacement the test proves nothing: each revision would parse.
    let stats = rig.src.session_stats();
    assert!(
        stats.applied < stats.revisions,
        "no revision was displaced: {stats:?}"
    );
}
