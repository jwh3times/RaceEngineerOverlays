use std::cell::{Cell, RefCell};
use std::rc::Rc;

use reo_irsdk::testing::{Image, ImageBuilder};
use reo_irsdk::{Client, Frame, SharedMemory, Update, VarType};

fn image() -> Image {
    ImageBuilder::new()
        .var_with_unit("Speed", VarType::Float, 1, "m/s")
        .var("Gear", VarType::Int, 1)
        .var("OnPitRoad", VarType::Bool, 1)
        .var("SessionTime", VarType::Double, 1)
        .var("SessionFlags", VarType::BitField, 1)
        .var("CarIdxLapDistPct", VarType::Float, 64)
        .build()
}

fn new_frame(update: Update) -> (bool, bool) {
    match update {
        Update::NewFrame {
            vars_changed,
            session_changed,
        } => (vars_changed, session_changed),
        other => panic!("expected a new frame, got {other:?}"),
    }
}

#[test]
fn reads_typed_values_from_the_newest_buffer() {
    let mut img = image();
    img.set(0, "Speed", 0, 10.0f32);
    img.set_tick(0, 100);
    img.set(1, "Speed", 0, 42.5f32);
    img.set(1, "Gear", 0, 3i32);
    img.set(1, "OnPitRoad", 0, true);
    img.set(1, "SessionTime", 0, 1234.5f64);
    img.set(1, "SessionFlags", 0, 0x0004u32);
    img.set(1, "CarIdxLapDistPct", 0, 0.25f32);
    img.set(1, "CarIdxLapDistPct", 63, 0.75f32);
    img.set_tick(1, 101);

    let mut client = Client::new(img.bytes);
    let mut frame = Frame::default();
    let (vars_changed, _) = new_frame(client.poll(&mut frame).unwrap());
    assert!(vars_changed);
    assert_eq!(frame.tick, 101);

    let vars = client.vars().clone();
    assert_eq!(vars.get("Speed").unwrap().unit, "m/s");
    let row = &frame.row;
    assert_eq!(vars.handle::<f32>("Speed").unwrap().get(row), Some(42.5));
    assert_eq!(vars.handle::<i32>("Gear").unwrap().get(row), Some(3));
    assert_eq!(
        vars.handle::<bool>("OnPitRoad").unwrap().get(row),
        Some(true)
    );
    assert_eq!(
        vars.handle::<f64>("SessionTime").unwrap().get(row),
        Some(1234.5)
    );
    assert_eq!(
        vars.handle::<u32>("SessionFlags").unwrap().get(row),
        Some(4)
    );

    let pct = vars.handle::<f32>("CarIdxLapDistPct").unwrap();
    assert_eq!(pct.count(), 64);
    assert_eq!(pct.at(row, 0), Some(0.25));
    assert_eq!(pct.at(row, 63), Some(0.75));
    assert_eq!(pct.at(row, 64), None);
    assert_eq!(pct.iter(row).count(), 64);
}

#[test]
fn rejects_handles_of_the_wrong_type_or_missing_names() {
    let mut img = image();
    img.set_tick(0, 1);
    let mut client = Client::new(img.bytes);
    client.poll(&mut Frame::default()).unwrap();
    let vars = client.vars();
    assert!(vars.handle::<i32>("Speed").is_none());
    assert!(vars.handle::<bool>("Gear").is_none());
    assert!(vars.handle::<f32>("NoSuchVar").is_none());
    // Floats widen to f64 for convenience.
    assert!(vars.handle::<f64>("Speed").is_some());
}

#[test]
fn reports_unchanged_until_a_newer_tick_arrives() {
    let mut img = image();
    img.set_tick(0, 5);
    let mut client = Client::new(img.bytes);
    let mut frame = Frame::default();

    let (vars_changed, _) = new_frame(client.poll(&mut frame).unwrap());
    assert!(vars_changed);
    assert_eq!(client.poll(&mut frame).unwrap(), Update::Unchanged);
}

/// An image the test can keep editing after handing it to a client.
#[derive(Clone)]
struct Shared(Rc<RefCell<Image>>);

impl Shared {
    fn new(img: Image) -> Self {
        Self(Rc::new(RefCell::new(img)))
    }
    fn edit(&self, f: impl FnOnce(&mut Image)) {
        f(&mut self.0.borrow_mut());
    }
}

impl SharedMemory for Shared {
    fn len(&self) -> usize {
        self.0.borrow().bytes.len()
    }
    fn read_into(&self, offset: usize, dst: &mut [u8]) -> bool {
        self.0.borrow().bytes.read_into(offset, dst)
    }
}

#[test]
fn reports_disconnected_and_reloads_layout_on_reconnect() {
    let mem = Shared::new(image());
    mem.edit(|img| img.set_tick(0, 5));
    let mut client = Client::new(mem.clone());
    let mut frame = Frame::default();
    let (vars_changed, _) = new_frame(client.poll(&mut frame).unwrap());
    assert!(vars_changed);

    mem.edit(|img| img.set_connected(false));
    assert_eq!(client.poll(&mut frame).unwrap(), Update::Disconnected);

    // Same tick as before the disconnect: a fresh connection must still deliver it.
    mem.edit(|img| img.set_connected(true));
    let (vars_changed, session_changed) = new_frame(client.poll(&mut frame).unwrap());
    assert!(vars_changed);
    assert!(session_changed);
    assert_eq!(frame.tick, 5);
}

#[test]
fn decodes_session_info_as_windows_1252_and_tracks_revisions() {
    let mut img = image();
    img.set_tick(0, 1);
    img.set_session_info("DriverInfo:\n UserName: Jürgen Müller\n", 7);
    let mut client = Client::new(img.bytes);
    let mut frame = Frame::default();

    let (_, session_changed) = new_frame(client.poll(&mut frame).unwrap());
    assert!(session_changed);
    let (text, revision) = client.session_info().unwrap();
    assert_eq!(revision, 7);
    assert!(text.contains("Jürgen Müller"), "{text}");
    assert!(!text.contains('\0'));
}

#[test]
fn session_changed_stays_set_until_session_info_is_read() {
    let mem = Shared::new(image());
    mem.edit(|img| {
        img.set_tick(0, 1);
        img.set_session_info("a: 1\n", 1);
    });
    let mut client = Client::new(mem.clone());
    let mut frame = Frame::default();

    assert!(new_frame(client.poll(&mut frame).unwrap()).1);
    mem.edit(|img| img.set_tick(1, 2));
    assert!(
        new_frame(client.poll(&mut frame).unwrap()).1,
        "not read yet"
    );

    client.session_info().unwrap();
    mem.edit(|img| img.set_tick(2, 3));
    assert!(!new_frame(client.poll(&mut frame).unwrap()).1);

    mem.edit(|img| {
        img.set_session_info("a: 2\n", 2);
        img.set_tick(0, 4);
    });
    assert!(new_frame(client.poll(&mut frame).unwrap()).1);
    assert_eq!(client.session_info().unwrap(), ("a: 2\n".to_owned(), 2));
}

type Rewrite = Box<dyn Fn(&mut Vec<u8>)>;

/// Memory that simulates the sim publishing a new tick into the same buffer
/// while the first row copy is in progress.
struct RacingWriter {
    bytes: RefCell<Vec<u8>>,
    row_reads: Cell<usize>,
    row_len: usize,
    rewrite: Rewrite,
}

impl SharedMemory for RacingWriter {
    fn len(&self) -> usize {
        self.bytes.borrow().len()
    }

    fn read_into(&self, offset: usize, dst: &mut [u8]) -> bool {
        let ok = self.bytes.borrow().read_into(offset, dst);
        if dst.len() == self.row_len {
            let n = self.row_reads.get() + 1;
            self.row_reads.set(n);
            if n == 1 {
                (self.rewrite)(&mut self.bytes.borrow_mut());
            }
        }
        ok
    }
}

#[test]
fn retries_when_the_row_is_rewritten_during_the_copy() {
    let mut first = image();
    first.set(0, "Gear", 0, 1i32);
    first.set_tick(0, 10);

    let mut second = image();
    second.set(0, "Gear", 0, 2i32);
    second.set_tick(0, 11);
    let rewritten = second.bytes;

    let row_len = first.row_len();
    let mem = RacingWriter {
        bytes: RefCell::new(first.bytes),
        row_reads: Cell::new(0),
        row_len,
        rewrite: Box::new(move |bytes| bytes.copy_from_slice(&rewritten)),
    };
    let mut client = Client::new(mem);
    let mut frame = Frame::default();
    new_frame(client.poll(&mut frame).unwrap());

    assert_eq!(client.memory().row_reads.get(), 2);
    assert_eq!(frame.tick, 11);
    let gear = client.vars().handle::<i32>("Gear").unwrap();
    assert_eq!(gear.get(&frame.row), Some(2));
}

#[test]
fn errors_on_truncated_memory_and_foreign_headers() {
    let mut client = Client::new(vec![0u8; 16]);
    assert!(client.poll(&mut Frame::default()).is_err());

    let mut img = image();
    img.set_tick(0, 1);
    img.bytes[0..4].copy_from_slice(&99i32.to_le_bytes());
    let mut client = Client::new(img.bytes);
    assert!(matches!(
        client.poll(&mut Frame::default()),
        Err(reo_irsdk::Error::UnsupportedVersion(99))
    ));
}

#[test]
fn drops_variables_that_overrun_the_row() {
    let mut img = image();
    img.set_tick(0, 1);
    // Point Speed (the first var header) past the end of the row.
    let row_len = img.row_len() as i32;
    let at = reo_irsdk::layout::HEADER_SIZE + 4;
    img.bytes[at..at + 4].copy_from_slice(&row_len.to_le_bytes());
    let mut client = Client::new(img.bytes);
    client.poll(&mut Frame::default()).unwrap();
    assert!(client.vars().get("Speed").is_none());
    assert_eq!(client.vars().rejected, vec!["Speed".to_owned()]);
    assert!(client.vars().get("Gear").is_some());
}
