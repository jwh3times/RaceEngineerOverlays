//! Parses iRacing's session-info YAML into [`SessionInfo`].
//!
//! iRacing writes free text (driver and team names) unquoted, so names such as
//! `[CLUB] Racing` or `Team: Fast` are not valid YAML. [`sanitize`] quotes such
//! values first. Fields are then read leniently: a missing or oddly typed
//! field becomes `None` rather than failing the whole document.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::time::Duration;

use reo_model::{
    CarClass, DriverEntry, PlayerCarSpec, SessionDefinition, SessionInfo, SessionKind, SimId,
    TrackInfo,
};
use serde_yaml_ng::Value;

#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "session info YAML: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(yaml: &str, revision: i32) -> Result<SessionInfo, ParseError> {
    let doc: Value =
        serde_yaml_ng::from_str(&sanitize(yaml)).map_err(|e| ParseError(e.to_string()))?;

    let weekend = field(&doc, "WeekendInfo");
    let driver_info = field(&doc, "DriverInfo");

    let track = TrackInfo {
        name: text(weekend, "TrackDisplayName")
            .or_else(|| text(weekend, "TrackName"))
            .unwrap_or_default(),
        config: text(weekend, "TrackConfigName"),
        length_m: text(weekend, "TrackLength").and_then(|t| distance_m(&t)),
    };

    let sessions = list(field(&doc, "SessionInfo"), "Sessions")
        .iter()
        .filter_map(|s| {
            let num = u16::try_from(int(Some(s), "SessionNum")?).ok()?;
            let kind_text = text(Some(s), "SessionType").unwrap_or_default();
            Some(SessionDefinition {
                num,
                kind: session_kind(&kind_text),
                name: text(Some(s), "SessionName").unwrap_or(kind_text),
                laps: int(Some(s), "SessionLaps").and_then(|l| u32::try_from(l).ok()),
                duration: float(Some(s), "SessionTime")
                    .filter(|&t| t > 0.0 && t < UNLIMITED_TIME_S)
                    .map(Duration::from_secs_f64),
            })
        })
        .collect();

    let mut drivers: Vec<_> = list(driver_info, "Drivers")
        .iter()
        .filter_map(|d| driver(Some(d)))
        .collect();
    name_classes(&mut drivers);

    Ok(SessionInfo {
        sim: Some(SimId::IRacing),
        revision: i64::from(revision),
        track,
        sessions,
        player_car_idx: int(driver_info, "DriverCarIdx").and_then(|i| u16::try_from(i).ok()),
        player_car: PlayerCarSpec {
            redline_rpm: float(driver_info, "DriverCarRedLine").map(|v| v as f32),
            shift_rpm: float(driver_info, "DriverCarSLShiftRPM")
                .filter(|&v| v > 0.0)
                .map(|v| v as f32),
            fuel_capacity_l: float(driver_info, "DriverCarFuelMaxLtr").map(|v| v as f32),
        },
        drivers,
    })
}

/// iRacing's marker for an untimed session (one week).
const UNLIMITED_TIME_S: f64 = 604_800.0;

fn driver(d: Option<&Value>) -> Option<DriverEntry> {
    Some(DriverEntry {
        car_idx: u16::try_from(int(d, "CarIdx")?).ok()?,
        name: text(d, "UserName").unwrap_or_default(),
        abbrev_name: text(d, "AbbrevName"),
        team_name: text(d, "TeamName"),
        car_number: text(d, "CarNumber").unwrap_or_default(),
        car_name: text(d, "CarScreenName").unwrap_or_default(),
        class: CarClass {
            id: int(d, "CarClassID")
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or(0),
            // Filled in per class by `name_classes` when iRacing leaves it empty.
            name: text(d, "CarClassShortName"),
            color: int(d, "CarClassColor").and_then(|v| u32::try_from(v).ok()),
            est_lap_time: float(d, "CarClassEstLapTime")
                .filter(|&t| t > 0.0)
                .map(|t| t as f32),
        },
        rating: int(d, "IRating").and_then(|v| i32::try_from(v).ok()),
        license: text(d, "LicString"),
        is_spectator: int(d, "IsSpectator").is_some_and(|v| v != 0),
        is_pace_car: int(d, "CarIsPaceCar").is_some_and(|v| v != 0),
    })
}

/// iRacing leaves `CarClassShortName` empty in AI and hosted sessions, single-
/// and multi-class alike. Name each unnamed class from what its drivers share:
/// a name any of them carries, else the car when the class has one model.
/// A class of several models stays `None`; its `id` still groups it.
fn name_classes(drivers: &mut [DriverEntry]) {
    let names: BTreeMap<i32, Option<String>> = drivers
        .iter()
        .map(|d| (d.class.id, class_name(drivers, d.class.id)))
        .collect();
    for d in drivers.iter_mut().filter(|d| d.class.name.is_none()) {
        d.class.name.clone_from(&names[&d.class.id]);
    }
}

fn class_name(drivers: &[DriverEntry], id: i32) -> Option<String> {
    let class = drivers.iter().filter(|d| d.class.id == id);
    if let Some(name) = class.clone().find_map(|d| d.class.name.clone()) {
        return Some(name);
    }
    let mut models = class.map(|d| d.car_name.as_str()).filter(|n| !n.is_empty());
    let model = models.next()?;
    models.all(|m| m == model).then(|| model.to_owned())
}

fn session_kind(kind: &str) -> SessionKind {
    let kind = kind.to_ascii_lowercase();
    if kind.contains("race") {
        SessionKind::Race
    } else if kind.contains("qualify") {
        SessionKind::Qualifying
    } else if kind.contains("warmup") {
        SessionKind::Warmup
    } else if kind.contains("practice") || kind.contains("testing") {
        SessionKind::Practice
    } else {
        SessionKind::Other
    }
}

/// Parses "6.93 km" or "4.31 mi" into metres.
fn distance_m(text: &str) -> Option<f32> {
    let value = leading_number(text)?;
    let scale = if text.trim_end().ends_with("mi") {
        1609.344
    } else {
        1000.0
    };
    Some((value * scale) as f32)
}

fn field<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_mapping()?.get(key)
}

fn list<'a>(v: Option<&'a Value>, key: &str) -> &'a [Value] {
    v.and_then(|v| field(v, key))
        .and_then(Value::as_sequence)
        .map_or(&[], Vec::as_slice)
}

/// Any scalar as trimmed text; empty becomes `None`.
fn text(v: Option<&Value>, key: &str) -> Option<String> {
    let s = match field(v?, key)? {
        Value::String(s) => s.trim().to_owned(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => return None,
    };
    (!s.is_empty()).then_some(s)
}

fn float(v: Option<&Value>, key: &str) -> Option<f64> {
    match field(v?, key)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => leading_number(s),
        _ => None,
    }
}

/// Integers, including hex such as `0xffda59` and numbers followed by units.
fn int(v: Option<&Value>, key: &str) -> Option<i64> {
    match field(v?, key)? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => {
            let s = s.trim();
            if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                i64::from_str_radix(hex, 16).ok()
            } else {
                s.parse()
                    .ok()
                    .or_else(|| leading_number(s).map(|f| f as i64))
            }
        }
        _ => None,
    }
}

fn leading_number(s: &str) -> Option<f64> {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || c == '.' || (i == 0 && (c == '-' || c == '+'))))
        .map_or(s.len(), |(i, _)| i);
    s[..end].parse().ok()
}

/// Single-quotes unquoted scalar values that would not parse as plain YAML.
pub fn sanitize(yaml: &str) -> Cow<'_, str> {
    let mut out: Option<String> = None;
    for (n, line) in yaml.split_inclusive('\n').enumerate() {
        let fixed = quote_value(line);
        match (&mut out, fixed) {
            (Some(buf), Some(f)) => buf.push_str(&f),
            (Some(buf), None) => buf.push_str(line),
            (None, Some(f)) => {
                let mut buf = String::with_capacity(yaml.len() + 64);
                for prev in yaml.split_inclusive('\n').take(n) {
                    buf.push_str(prev);
                }
                buf.push_str(&f);
                out = Some(buf);
            }
            (None, None) => {}
        }
    }
    out.map_or(Cow::Borrowed(yaml), Cow::Owned)
}

fn quote_value(line: &str) -> Option<String> {
    let body = line.trim_end_matches(['\n', '\r']);
    let ending = &line[body.len()..];
    let indent = body.len() - body.trim_start().len();
    let mut rest = &body[indent..];
    let dash = if let Some(r) = rest.strip_prefix("- ") {
        rest = r;
        "- "
    } else {
        ""
    };
    let colon = rest.find(": ")?;
    let key = &rest[..colon];
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let value = rest[colon + 2..].trim();
    if value.is_empty() || !needs_quoting(value) {
        return None;
    }
    Some(format!(
        "{}{dash}{key}: '{}'{ending}",
        &body[..indent],
        value.replace('\'', "''")
    ))
}

fn needs_quoting(v: &str) -> bool {
    let first = v.chars().next().unwrap_or(' ');
    if first == '"' || first == '\'' {
        // Leave a value that is already one well-formed quoted scalar.
        let well_formed = v.len() >= 2 && v.ends_with(first) && !v[1..v.len() - 1].contains(first);
        return !well_formed;
    }
    matches!(
        first,
        '[' | ']' | '{' | '}' | '&' | '*' | '!' | '|' | '>' | '%' | '@' | '`' | ',' | '#'
    ) || v == "-"
        || v == "?"
        || v.starts_with("- ")
        || v.starts_with("? ")
        || v.starts_with(": ")
        || v.contains(": ")
        || v.contains(" #")
        || v.ends_with(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_values_that_break_plain_yaml() {
        let src = "DriverInfo:\n Drivers:\n - CarIdx: 1\n   UserName: [CLUB] Bob\n   TeamName: Team: Fast\n   AbbrevName: O'Neil, J\n   CarNumber: \"07\"\n   IRating: 1500\n";
        let out = sanitize(src);
        assert!(out.contains("   UserName: '[CLUB] Bob'\n"), "{out}");
        assert!(out.contains("   TeamName: 'Team: Fast'\n"), "{out}");
        assert!(out.contains("   AbbrevName: O'Neil, J\n"), "{out}");
        assert!(out.contains("   CarNumber: \"07\"\n"), "{out}");
        assert!(out.contains(" - CarIdx: 1\n"), "{out}");
        assert!(out.contains("   IRating: 1500\n"), "{out}");
    }

    #[test]
    fn leaves_clean_documents_untouched() {
        let src = "WeekendInfo:\n TrackName: spa\n";
        assert!(matches!(sanitize(src), Cow::Borrowed(_)));
    }

    #[test]
    fn escapes_embedded_single_quotes() {
        let out = sanitize(" - UserName: *Bob's* car\n");
        assert_eq!(out, " - UserName: '*Bob''s* car'\n");
    }

    /// One driver as iRacing writes it, with an empty `class` written as the
    /// bare `CarClassShortName: ` iRacing leaves in AI and hosted sessions.
    fn driver_yaml(car_idx: u16, class_id: i32, car: &str, class: &str) -> String {
        format!(
            " - CarIdx: {car_idx}\n   CarScreenName: {car}\n   CarClassID: {class_id}\n   CarClassShortName: {class}\n"
        )
    }

    fn class_names(drivers: &[String]) -> Vec<Option<String>> {
        let yaml = format!("DriverInfo:\n Drivers:\n{}", drivers.concat());
        let info = parse(&yaml, 1).unwrap();
        info.drivers.into_iter().map(|d| d.class.name).collect()
    }

    #[test]
    fn an_unnamed_single_model_class_takes_its_car_name() {
        let names = class_names(&[
            driver_yaml(1, 2523, "Dallara P217 LMP2", ""),
            driver_yaml(2, 2523, "Dallara P217 LMP2", ""),
        ]);
        assert_eq!(
            names,
            [
                Some("Dallara P217 LMP2".into()),
                Some("Dallara P217 LMP2".into())
            ]
        );
    }

    #[test]
    fn an_unnamed_multi_model_class_has_no_name() {
        let names = class_names(&[
            driver_yaml(1, 4011, "BMW M4 GT3 EVO", ""),
            driver_yaml(2, 4011, "Ferrari 296 GT3", ""),
            driver_yaml(3, 2523, "Dallara P217 LMP2", ""),
        ]);
        assert_eq!(names, [None, None, Some("Dallara P217 LMP2".into())]);
    }

    #[test]
    fn a_name_one_driver_supplies_names_the_whole_class() {
        let names = class_names(&[
            driver_yaml(1, 4011, "BMW M4 GT3 EVO", ""),
            driver_yaml(2, 4011, "Ferrari 296 GT3", "GT3"),
        ]);
        assert_eq!(names, [Some("GT3".into()), Some("GT3".into())]);
    }

    #[test]
    fn named_classes_keep_iracings_names() {
        let names = class_names(&[
            driver_yaml(1, 4011, "Porsche 911 GT3 R (992)", "GT3 Class"),
            driver_yaml(2, 2523, "Dallara P217 LMP2", "LMP2"),
            // A driver's own name is never replaced by another driver's.
            driver_yaml(3, 4011, "BMW M4 GT3 EVO", "GT3 Pro"),
        ]);
        assert_eq!(
            names,
            [
                Some("GT3 Class".into()),
                Some("LMP2".into()),
                Some("GT3 Pro".into())
            ]
        );
    }

    #[test]
    fn a_driver_with_no_car_name_does_not_split_the_class() {
        let names = class_names(&[
            driver_yaml(1, 2523, "", ""),
            driver_yaml(2, 2523, "Dallara P217 LMP2", ""),
        ]);
        let lmp2 = Some("Dallara P217 LMP2".to_owned());
        assert_eq!(names, [lmp2.clone(), lmp2]);
    }

    #[test]
    fn a_class_with_no_names_at_all_is_none_not_empty() {
        let names = class_names(&[driver_yaml(1, 4011, "", "")]);
        assert_eq!(names, [None]);
    }

    #[test]
    fn parses_units_and_hex() {
        assert_eq!(leading_number("1800.0000 sec"), Some(1800.0));
        assert_eq!(leading_number("-1.5"), Some(-1.5));
        assert_eq!(leading_number("unlimited"), None);
        assert!((distance_m("6.93 km").unwrap() - 6930.0).abs() < 0.5);
        assert!((distance_m("2.50 mi").unwrap() - 4023.36).abs() < 0.5);
    }
}
