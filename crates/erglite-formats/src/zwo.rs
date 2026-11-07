// SPDX-License-Identifier: MIT OR Apache-2.0
//! Zwift `.zwo` workout import.

use crate::FormatError;
use erglite_core::workout::{
    Control, End, PowerTarget, RepeatBlock, SCHEMA_VERSION, Segment, SegmentNode, Workout,
    WorkoutKind,
};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// Result of an import, with non-fatal warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportReport {
    /// The imported workout.
    pub workout: Workout,
    /// Elements that were skipped or adjusted.
    pub warnings: Vec<String>,
}

fn attr(e: &BytesStart<'_>, name: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == name.as_bytes())
        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
}

fn num(e: &BytesStart<'_>, name: &str) -> Option<f32> {
    attr(e, name).and_then(|v| v.trim().parse().ok())
}

fn seg(label: &str, seconds: f32, control: Control) -> Segment {
    Segment {
        label: Some(label.to_string()),
        end: End::TimeS(seconds.round().max(1.0) as u32),
        control,
        cadence_rpm: None,
        message: None,
    }
}

fn pct(v: f32) -> f32 {
    v * 100.0
}

/// Import a `.zwo` document. Power values in `.zwo` are fractions of FTP.
pub fn import_zwo(xml: &str) -> Result<ImportReport, FormatError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let (mut name, mut description) = (String::new(), None::<String>);
    let mut nodes: Vec<SegmentNode> = Vec::new();
    let mut warnings = Vec::new();
    let mut current_text: Option<String> = None;
    loop {
        match reader.read_event()? {
            Event::Start(e) | Event::Empty(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let dur = num(&e, "Duration").unwrap_or(0.0);
                match tag.as_str() {
                    "name" | "description" => current_text = Some(tag),
                    "Warmup" | "Cooldown" | "Ramp" => {
                        let (from, to) = (num(&e, "PowerLow"), num(&e, "PowerHigh"));
                        if let (Some(a), Some(b)) = (from, to) {
                            let label = match tag.as_str() {
                                "Warmup" => "Warm-up",
                                "Cooldown" => "Cool-down",
                                _ => "Ramp",
                            };
                            let target = PowerTarget::ramp_pct(pct(a), pct(b));
                            nodes.push(SegmentNode::Single(seg(
                                label,
                                dur,
                                Control::Power(target),
                            )));
                        } else {
                            warnings.push(format!("{tag} without PowerLow/PowerHigh skipped"));
                        }
                    }
                    "SteadyState" => match num(&e, "Power") {
                        Some(p) => nodes.push(SegmentNode::Single(seg(
                            "Steady",
                            dur,
                            Control::Power(PowerTarget::steady_pct(pct(p))),
                        ))),
                        None => warnings.push("SteadyState without Power skipped".into()),
                    },
                    "IntervalsT" => {
                        let on = (num(&e, "OnDuration"), num(&e, "OnPower"));
                        let off = (num(&e, "OffDuration"), num(&e, "OffPower"));
                        if let ((Some(od), Some(op)), (Some(fd), Some(fp))) = (on, off) {
                            nodes.push(SegmentNode::Repeat(RepeatBlock {
                                repeat: num(&e, "Repeat").unwrap_or(1.0).max(1.0) as u32,
                                segments: vec![
                                    seg(
                                        "Work",
                                        od,
                                        Control::Power(PowerTarget::steady_pct(pct(op))),
                                    ),
                                    seg(
                                        "Rest",
                                        fd,
                                        Control::Power(PowerTarget::steady_pct(pct(fp))),
                                    ),
                                ],
                            }));
                        } else {
                            warnings.push("IntervalsT with missing attributes skipped".into());
                        }
                    }
                    "FreeRide" => nodes.push(SegmentNode::Single(seg(
                        "Free ride",
                        dur.max(1.0),
                        Control::Free,
                    ))),
                    "workout_file" | "author" | "tags" | "tag" | "sportType" | "workout" => {}
                    other => warnings.push(format!("unsupported element <{other}> skipped")),
                }
            }
            Event::Text(t) => {
                if let Some(which) = current_text.take() {
                    let text = t
                        .decode()
                        .map_err(|e| FormatError::Xml(e.to_string()))?
                        .into_owned();
                    if which == "name" {
                        name = text;
                    } else {
                        description = Some(text);
                    }
                }
            }
            Event::End(_) => current_text = None,
            Event::Eof => break,
            _ => {}
        }
    }
    if nodes.is_empty() {
        return Err(FormatError::Invalid(
            "no supported workout elements found".into(),
        ));
    }
    let workout = Workout {
        schema_version: SCHEMA_VERSION,
        name: if name.is_empty() {
            "Imported workout".into()
        } else {
            name
        },
        description,
        kind: WorkoutKind::Time,
        route: None,
        segments: nodes,
    };
    workout
        .validate()
        .map_err(|e| FormatError::Invalid(e.to_string()))?;
    Ok(ImportReport { workout, warnings })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const ZWO: &str = r#"<workout_file><author>x</author><name>Sweet Spot</name>
      <description>3 x 8</description><sportType>bike</sportType><tags/>
      <workout>
        <Warmup Duration="600" PowerLow="0.45" PowerHigh="0.70"/>
        <IntervalsT Repeat="3" OnDuration="480" OffDuration="240" OnPower="0.95" OffPower="0.55"/>
        <SteadyState Duration="300" Power="0.60"/>
        <FreeRide Duration="120"/>
        <Cooldown Duration="600" PowerLow="0.60" PowerHigh="0.40"/>
        <Surprise Duration="1"/>
      </workout></workout_file>"#;

    #[test]
    fn imports_a_typical_workout() {
        let r = import_zwo(ZWO).unwrap();
        assert_eq!(r.workout.name, "Sweet Spot");
        assert_eq!(r.workout.description.as_deref(), Some("3 x 8"));
        assert_eq!(r.workout.flatten().len(), 1 + 6 + 1 + 1 + 1);
        assert_eq!(r.workout.planned_time_s(), 600 + 3 * 720 + 300 + 120 + 600);
        assert_eq!(r.warnings.len(), 1);
        assert!(r.warnings[0].contains("Surprise"));
    }

    #[test]
    fn empty_workout_is_rejected() {
        assert!(import_zwo("<workout_file><workout></workout></workout_file>").is_err());
        assert!(import_zwo("<workout_file><oops></workout_file>").is_err());
    }
}
