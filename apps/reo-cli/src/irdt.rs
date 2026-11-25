//! Plays irDashies `.irdt` telemetry tapes into an in-process SDK image.
//!
//! The tape holds the original SDK header and variable table, then a stream of
//! checksummed records: telemetry rows, session-info revisions, gaps and
//! disconnects. Applying the records in order reproduces what iRacing's shared
//! memory looked like at each tick, so the real reader and adapter can run
//! against it without the sim.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use reo_irsdk::SharedBuffer;
use reo_irsdk::layout::{
    HEADER_SIZE, Header, MAX_BUFS, STATUS_CONNECTED, VAR_HEADER_SIZE, tick_count_offset,
};

const MAGIC: &[u8; 8] = b"IRDTRCE\0";
const FORMAT_VERSION: u32 = 1;
const ENDIAN_MARKER: u32 = 0x0102_0304;
const FILE_HEADER_SIZE: usize = 96;
const RECORD_HEADER_SIZE: usize = 40;
const MAX_MAPPING_SIZE: u64 = 512 * 1024 * 1024;
const MAX_PAYLOAD_SIZE: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Record {
    /// A telemetry row was published.
    Frame {
        elapsed_s: f64,
    },
    /// The session-info YAML was revised.
    SessionInfo,
    Gap,
    Disconnect,
    End,
}

pub struct IrdtTape {
    reader: BufReader<File>,
    image: SharedBuffer,
    header: Header,
    qpc_frequency: u64,
    pub record_count: u64,
    payload: Vec<u8>,
}

impl IrdtTape {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mut reader = BufReader::with_capacity(1 << 20, file);

        let mut fh = [0u8; FILE_HEADER_SIZE];
        reader.read_exact(&mut fh).context("reading tape header")?;
        ensure!(&fh[0..8] == MAGIC, "not an irDashies telemetry tape");
        ensure!(
            u32_at(&fh, 8) == FORMAT_VERSION,
            "unsupported tape version {}",
            u32_at(&fh, 8)
        );
        ensure!(
            u32_at(&fh, 12) == ENDIAN_MARKER,
            "tape has the wrong byte order"
        );
        ensure!(
            u32_at(&fh, 16) as usize == FILE_HEADER_SIZE
                && u32_at(&fh, 20) as usize == HEADER_SIZE
                && u32_at(&fh, 24) as usize == VAR_HEADER_SIZE,
            "tape struct sizes don't match SDK version 2"
        );
        let var_count = u32_at(&fh, 28) as usize;
        let mapping_size = u64_at(&fh, 32);
        let qpc_frequency = u64_at(&fh, 40);
        let record_count = u64_at(&fh, 48);
        let schema_checksum = u32_at(&fh, 56);
        ensure!(
            (1..=4096).contains(&var_count),
            "implausible variable count {var_count}"
        );
        ensure!(
            mapping_size > 0 && mapping_size <= MAX_MAPPING_SIZE,
            "implausible mapping size"
        );
        ensure!(qpc_frequency > 0, "tape has no clock frequency");

        let mut sdk = [0u8; HEADER_SIZE];
        reader.read_exact(&mut sdk).context("reading SDK header")?;
        let mut vars = vec![0u8; var_count * VAR_HEADER_SIZE];
        reader
            .read_exact(&mut vars)
            .context("reading variable headers")?;
        ensure!(
            fnv1a(&vars) == schema_checksum,
            "variable schema checksum mismatch"
        );

        let header = Header::parse(&sdk);
        ensure!(
            (1..=MAX_BUFS as i32).contains(&header.num_buf),
            "implausible buffer count {}",
            header.num_buf
        );

        let image = SharedBuffer::new(vec![0u8; mapping_size as usize]);
        ensure!(image.write(0, &sdk), "SDK header outside the mapping");
        ensure!(
            image.write(header.var_header_offset as usize, &vars),
            "variable headers outside the mapping"
        );
        // Start the way the replay publisher does: connected, no session info, no ticks.
        image.write_i32(4, STATUS_CONNECTED);
        image.write_i32(12, -1);
        image.write_i32(16, 0);
        for i in 0..header.buffer_count() {
            image.write_i32(tick_count_offset(i), -1);
        }

        Ok(Self {
            reader,
            image,
            header,
            qpc_frequency,
            record_count,
            payload: Vec::new(),
        })
    }

    /// The image a [`reo_irsdk::Client`] should read.
    pub fn memory(&self) -> SharedBuffer {
        self.image.clone()
    }

    /// Reads the next record and applies it to the image. `None` at end of file.
    pub fn next_record(&mut self) -> Result<Option<Record>> {
        let mut rh = [0u8; RECORD_HEADER_SIZE];
        match self.reader.read_exact(&mut rh) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e).context("reading record header"),
        }
        let kind = u32_at(&rh, 0);
        ensure!(
            u32_at(&rh, 4) as usize == RECORD_HEADER_SIZE,
            "bad record header size"
        );
        let payload_size = u32_at(&rh, 8) as usize;
        ensure!(payload_size <= MAX_PAYLOAD_SIZE, "record payload too large");
        let elapsed_ticks = u64_at(&rh, 16);
        let source_tick = i32_at(&rh, 24);
        let value = i32_at(&rh, 28);
        let checksum = u32_at(&rh, 32);

        self.payload.resize(payload_size, 0);
        self.reader
            .read_exact(&mut self.payload)
            .context("reading record payload")?;
        ensure!(fnv1a(&self.payload) == checksum, "record checksum mismatch");

        Ok(Some(match kind {
            1 => {
                let index = usize::try_from(value)
                    .ok()
                    .filter(|&i| i < self.header.buffer_count())
                    .context("frame record names an invalid buffer")?;
                ensure!(
                    self.payload.len() == self.header.buf_len as usize,
                    "frame length doesn't match the SDK row length"
                );
                let offset = self.header.var_bufs[index].buf_offset as usize;
                ensure!(
                    self.image.write(offset, &self.payload),
                    "frame outside the mapping"
                );
                self.image.write_i32(4, STATUS_CONNECTED);
                self.image.write_i32(tick_count_offset(index), source_tick);
                Record::Frame {
                    elapsed_s: elapsed_ticks as f64 / self.qpc_frequency as f64,
                }
            }
            2 => {
                // Recorders may capture the whole session region, so the payload can
                // fill it exactly. Zero-fill the rest of the region instead of writing
                // a terminator past its end, which would corrupt the variable headers.
                let offset = self.header.session_info_offset as usize;
                let region = (self.header.session_info_len.max(0) as usize).max(self.payload.len());
                let mut text = std::mem::take(&mut self.payload);
                text.resize(region, 0);
                let ok = self.image.write(offset, &text);
                text.truncate(u32_at(&rh, 8) as usize);
                self.payload = text;
                ensure!(ok, "session info outside the mapping");
                self.image.write_i32(16, self.payload.len() as i32);
                self.image.write_i32(12, value);
                Record::SessionInfo
            }
            3 => Record::Gap,
            4 => {
                self.image.write_i32(4, 0);
                Record::Disconnect
            }
            5 => Record::End,
            other => bail!("unknown record kind {other}"),
        }))
    }
}

fn fnv1a(bytes: &[u8]) -> u32 {
    let mut h: u32 = 2_166_136_261;
    for &b in bytes {
        h ^= u32::from(b);
        h = h.wrapping_mul(16_777_619);
    }
    h
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn i32_at(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reo_irsdk::testing::ImageBuilder;
    use reo_irsdk::{Client, Frame, Update, VarType};

    /// Writes a tape whose session payload fills the whole session region,
    /// as real recordings do.
    fn write_tape(path: &Path) -> Vec<u8> {
        let img = ImageBuilder::new()
            .var("SessionTime", VarType::Double, 1)
            .var("Gear", VarType::Int, 1)
            .session_capacity(64)
            .build();
        let mut sdk = img.bytes[..HEADER_SIZE].to_vec();
        let header = Header::parse(sdk.as_slice().try_into().unwrap());
        let vars = img.bytes[header.var_header_offset as usize..][..2 * VAR_HEADER_SIZE].to_vec();
        // Match iRacing's layout: a 64-byte session region directly followed by the
        // variable headers (bytes 176..464), then the rows.
        sdk[16..20].copy_from_slice(&64i32.to_le_bytes());
        sdk[20..24].copy_from_slice(&(HEADER_SIZE as i32).to_le_bytes());
        sdk[28..32].copy_from_slice(&((HEADER_SIZE + 64) as i32).to_le_bytes());
        assert_eq!(
            HEADER_SIZE + 64 + vars.len(),
            header.var_bufs[0].buf_offset as usize
        );

        let mut file = Vec::new();
        file.extend_from_slice(MAGIC);
        for v in [
            FORMAT_VERSION,
            ENDIAN_MARKER,
            FILE_HEADER_SIZE as u32,
            HEADER_SIZE as u32,
            VAR_HEADER_SIZE as u32,
            2,
        ] {
            file.extend_from_slice(&v.to_le_bytes());
        }
        file.extend_from_slice(&(img.bytes.len() as u64).to_le_bytes());
        file.extend_from_slice(&1000u64.to_le_bytes());
        file.extend_from_slice(&2u64.to_le_bytes());
        file.extend_from_slice(&fnv1a(&vars).to_le_bytes());
        file.resize(FILE_HEADER_SIZE, 0);
        file.extend_from_slice(&sdk);
        file.extend_from_slice(&vars);

        let mut session = b"WeekendInfo:\n TrackName: x\n".to_vec();
        session.resize(64, 0);
        let mut row = vec![0u8; header.buf_len as usize];
        row[0..8].copy_from_slice(&12.5f64.to_le_bytes());
        row[8..12].copy_from_slice(&3i32.to_le_bytes());
        for (kind, tick, value, payload) in [(2u32, 0i32, 1i32, &session), (1, 77, 0, &row)] {
            let mut rh = [0u8; RECORD_HEADER_SIZE];
            rh[0..4].copy_from_slice(&kind.to_le_bytes());
            rh[4..8].copy_from_slice(&(RECORD_HEADER_SIZE as u32).to_le_bytes());
            rh[8..12].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            rh[16..24].copy_from_slice(&500u64.to_le_bytes());
            rh[24..28].copy_from_slice(&tick.to_le_bytes());
            rh[28..32].copy_from_slice(&value.to_le_bytes());
            rh[32..36].copy_from_slice(&fnv1a(payload).to_le_bytes());
            file.extend_from_slice(&rh);
            file.extend_from_slice(payload);
        }
        std::fs::write(path, &file).unwrap();
        file
    }

    #[test]
    fn plays_a_tape_without_corrupting_the_variable_table() {
        let path = std::env::temp_dir().join(format!("reo-irdt-test-{}.irdt", std::process::id()));
        write_tape(&path);
        let mut tape = IrdtTape::open(&path).unwrap();
        assert_eq!(tape.next_record().unwrap(), Some(Record::SessionInfo));
        assert_eq!(
            tape.next_record().unwrap(),
            Some(Record::Frame { elapsed_s: 0.5 })
        );
        assert_eq!(tape.next_record().unwrap(), None);
        std::fs::remove_file(&path).ok();

        let mut client = Client::new(tape.memory());
        let mut frame = Frame::default();
        assert!(matches!(
            client.poll(&mut frame).unwrap(),
            Update::NewFrame { .. }
        ));
        assert_eq!(frame.tick, 77);
        let vars = client.vars().clone();
        assert_eq!(vars.get("SessionTime").unwrap().ty, VarType::Double);
        assert_eq!(
            vars.handle::<f64>("SessionTime").unwrap().get(&frame.row),
            Some(12.5)
        );
        assert_eq!(vars.handle::<i32>("Gear").unwrap().get(&frame.row), Some(3));
        let (yaml, revision) = client.session_info().unwrap();
        assert_eq!(
            (yaml.as_str(), revision),
            ("WeekendInfo:\n TrackName: x\n", 1)
        );
    }
}
