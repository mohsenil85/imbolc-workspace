//! Automation lane types for parameter automation.

use serde::{Deserialize, Serialize};

use crate::{EffectId, InstrumentId};

/// Unique identifier for an automation lane.
pub type AutomationLaneId = u32;

/// Interpolation curve type between automation points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurveType {
    /// Linear interpolation (default)
    Linear,
    /// Exponential curve (good for volume, frequency)
    Exponential,
    /// Instant jump (no interpolation)
    Step,
    /// S-curve (smooth transitions)
    SCurve,
}

impl Default for CurveType {
    fn default() -> Self {
        Self::Linear
    }
}

/// A single automation point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationPoint {
    /// Position in ticks
    pub tick: u32,
    /// Normalized value (0.0-1.0, mapped to param's min/max)
    pub value: f32,
    /// Curve type to next point
    pub curve: CurveType,
}

impl AutomationPoint {
    pub fn new(tick: u32, value: f32) -> Self {
        Self {
            tick,
            value: value.clamp(0.0, 1.0),
            curve: CurveType::default(),
        }
    }

    pub fn with_curve(tick: u32, value: f32, curve: CurveType) -> Self {
        Self {
            tick,
            value: value.clamp(0.0, 1.0),
            curve,
        }
    }
}

/// What parameter is being automated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AutomationTarget {
    /// Instrument output level
    InstrumentLevel(InstrumentId),
    /// Instrument pan
    InstrumentPan(InstrumentId),
    /// Filter cutoff frequency
    FilterCutoff(InstrumentId),
    /// Filter resonance
    FilterResonance(InstrumentId),
    /// Effect parameter (instrument_id, effect_id, param_index)
    EffectParam(InstrumentId, EffectId, usize),
    /// Sample playback rate (for scratching)
    SampleRate(InstrumentId),
    /// Sample amplitude
    SampleAmp(InstrumentId),
    /// LFO rate (0.1-32.0 Hz)
    LfoRate(InstrumentId),
    /// LFO depth (0.0-1.0)
    LfoDepth(InstrumentId),
    /// Envelope attack time (0.001-2.0 s)
    EnvelopeAttack(InstrumentId),
    /// Envelope decay time (0.001-2.0 s)
    EnvelopeDecay(InstrumentId),
    /// Envelope sustain level (0.0-1.0)
    EnvelopeSustain(InstrumentId),
    /// Envelope release time (0.001-5.0 s)
    EnvelopeRelease(InstrumentId),
    /// Send level (instrument_id, send_index, 0.0-1.0)
    SendLevel(InstrumentId, usize),
    /// Bus output level (bus 1-8, 0.0-1.0)
    BusLevel(u8),
    /// Global BPM (30.0-300.0)
    Bpm,
    /// VST plugin parameter (instrument_id, param_index, 0.0-1.0 normalized)
    VstParam(InstrumentId, u32),
    /// EQ band parameter (instrument_id, band_index 0-11, param: 0=freq 1=gain 2=q)
    EqBandParam(InstrumentId, usize, usize),
    /// Per-track swing amount (0.0-1.0)
    TrackSwing(InstrumentId),
    /// Per-track velocity humanization (0.0-1.0)
    TrackHumanizeVelocity(InstrumentId),
    /// Per-track timing humanization (0.0-1.0)
    TrackHumanizeTiming(InstrumentId),
    /// Per-track timing offset (-50.0 to +50.0 ms)
    TrackTimingOffset(InstrumentId),
}

impl AutomationTarget {
    /// Get the instrument ID associated with this target (None for global targets).
    pub fn instrument_id(&self) -> Option<InstrumentId> {
        match self {
            AutomationTarget::InstrumentLevel(id)
            | AutomationTarget::InstrumentPan(id)
            | AutomationTarget::FilterCutoff(id)
            | AutomationTarget::FilterResonance(id)
            | AutomationTarget::SampleRate(id)
            | AutomationTarget::SampleAmp(id)
            | AutomationTarget::LfoRate(id)
            | AutomationTarget::LfoDepth(id)
            | AutomationTarget::EnvelopeAttack(id)
            | AutomationTarget::EnvelopeDecay(id)
            | AutomationTarget::EnvelopeSustain(id)
            | AutomationTarget::EnvelopeRelease(id) => Some(*id),
            AutomationTarget::EffectParam(id, _, _) => Some(*id),
            AutomationTarget::SendLevel(id, _) => Some(*id),
            AutomationTarget::VstParam(id, _) => Some(*id),
            AutomationTarget::EqBandParam(id, _, _) => Some(*id),
            AutomationTarget::TrackSwing(id)
            | AutomationTarget::TrackHumanizeVelocity(id)
            | AutomationTarget::TrackHumanizeTiming(id)
            | AutomationTarget::TrackTimingOffset(id) => Some(*id),
            AutomationTarget::BusLevel(_) | AutomationTarget::Bpm => None,
        }
    }

    /// Get a human-readable name for this target.
    pub fn name(&self) -> String {
        match self {
            AutomationTarget::InstrumentLevel(_) => "Level".to_string(),
            AutomationTarget::InstrumentPan(_) => "Pan".to_string(),
            AutomationTarget::FilterCutoff(_) => "Filter Cutoff".to_string(),
            AutomationTarget::FilterResonance(_) => "Filter Resonance".to_string(),
            AutomationTarget::EffectParam(_, fx_idx, param_idx) => {
                format!("FX{} Param{}", fx_idx + 1, param_idx + 1)
            }
            AutomationTarget::SampleRate(_) => "Sample Rate".to_string(),
            AutomationTarget::SampleAmp(_) => "Sample Amp".to_string(),
            AutomationTarget::LfoRate(_) => "LFO Rate".to_string(),
            AutomationTarget::LfoDepth(_) => "LFO Depth".to_string(),
            AutomationTarget::EnvelopeAttack(_) => "Env Attack".to_string(),
            AutomationTarget::EnvelopeDecay(_) => "Env Decay".to_string(),
            AutomationTarget::EnvelopeSustain(_) => "Env Sustain".to_string(),
            AutomationTarget::EnvelopeRelease(_) => "Env Release".to_string(),
            AutomationTarget::SendLevel(_, idx) => format!("Send {}", idx + 1),
            AutomationTarget::BusLevel(bus) => format!("Bus {} Level", bus),
            AutomationTarget::Bpm => "BPM".to_string(),
            AutomationTarget::VstParam(_, idx) => format!("VST P{}", idx),
            AutomationTarget::EqBandParam(_, band, param) => {
                let param_name = match param {
                    0 => "Freq",
                    1 => "Gain",
                    _ => "Q",
                };
                format!("EQ B{} {}", band + 1, param_name)
            }
            AutomationTarget::TrackSwing(_) => "Track Swing".to_string(),
            AutomationTarget::TrackHumanizeVelocity(_) => "Track Humanize Vel".to_string(),
            AutomationTarget::TrackHumanizeTiming(_) => "Track Humanize Time".to_string(),
            AutomationTarget::TrackTimingOffset(_) => "Track Timing Offset".to_string(),
        }
    }

    /// Get a short name for compact display.
    pub fn short_name(&self) -> &'static str {
        match self {
            AutomationTarget::InstrumentLevel(_) => "Level",
            AutomationTarget::InstrumentPan(_) => "Pan",
            AutomationTarget::FilterCutoff(_) => "FltCt",
            AutomationTarget::FilterResonance(_) => "FltRs",
            AutomationTarget::EffectParam(_, _, _) => "FX",
            AutomationTarget::SampleRate(_) => "SRate",
            AutomationTarget::SampleAmp(_) => "SAmp",
            AutomationTarget::LfoRate(_) => "LfoRt",
            AutomationTarget::LfoDepth(_) => "LfoDp",
            AutomationTarget::EnvelopeAttack(_) => "EnvA",
            AutomationTarget::EnvelopeDecay(_) => "EnvD",
            AutomationTarget::EnvelopeSustain(_) => "EnvS",
            AutomationTarget::EnvelopeRelease(_) => "EnvR",
            AutomationTarget::SendLevel(_, _) => "Send",
            AutomationTarget::BusLevel(_) => "BusLv",
            AutomationTarget::Bpm => "BPM",
            AutomationTarget::VstParam(_, _) => "VstP",
            AutomationTarget::EqBandParam(_, _, _) => "EqBd",
            AutomationTarget::TrackSwing(_) => "TkSwg",
            AutomationTarget::TrackHumanizeVelocity(_) => "TkHVl",
            AutomationTarget::TrackHumanizeTiming(_) => "TkHTm",
            AutomationTarget::TrackTimingOffset(_) => "TkOfs",
        }
    }

    /// Get all possible automation targets for an instrument (static set).
    pub fn targets_for_instrument(id: InstrumentId) -> Vec<AutomationTarget> {
        vec![
            AutomationTarget::InstrumentLevel(id),
            AutomationTarget::InstrumentPan(id),
            AutomationTarget::FilterCutoff(id),
            AutomationTarget::FilterResonance(id),
            AutomationTarget::LfoRate(id),
            AutomationTarget::LfoDepth(id),
            AutomationTarget::EnvelopeAttack(id),
            AutomationTarget::EnvelopeDecay(id),
            AutomationTarget::EnvelopeSustain(id),
            AutomationTarget::EnvelopeRelease(id),
            AutomationTarget::TrackSwing(id),
            AutomationTarget::TrackHumanizeVelocity(id),
            AutomationTarget::TrackHumanizeTiming(id),
            AutomationTarget::TrackTimingOffset(id),
        ]
    }

    /// Normalize an actual parameter value to 0.0-1.0 based on this target's range.
    pub fn normalize_value(&self, actual: f32) -> f32 {
        let (min, max) = self.default_range();
        if max > min {
            ((actual - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.5
        }
    }

    /// Get the default min/max range for this target type.
    pub fn default_range(&self) -> (f32, f32) {
        match self {
            AutomationTarget::InstrumentLevel(_) => (0.0, 1.0),
            AutomationTarget::InstrumentPan(_) => (-1.0, 1.0),
            AutomationTarget::FilterCutoff(_) => (20.0, 20000.0),
            AutomationTarget::FilterResonance(_) => (0.0, 1.0),
            AutomationTarget::EffectParam(_, _, _) => (0.0, 1.0),
            AutomationTarget::SampleRate(_) => (-2.0, 2.0),
            AutomationTarget::SampleAmp(_) => (0.0, 1.0),
            AutomationTarget::LfoRate(_) => (0.1, 32.0),
            AutomationTarget::LfoDepth(_) => (0.0, 1.0),
            AutomationTarget::EnvelopeAttack(_) => (0.001, 2.0),
            AutomationTarget::EnvelopeDecay(_) => (0.001, 2.0),
            AutomationTarget::EnvelopeSustain(_) => (0.0, 1.0),
            AutomationTarget::EnvelopeRelease(_) => (0.001, 5.0),
            AutomationTarget::SendLevel(_, _) => (0.0, 1.0),
            AutomationTarget::BusLevel(_) => (0.0, 1.0),
            AutomationTarget::Bpm => (30.0, 300.0),
            AutomationTarget::VstParam(_, _) => (0.0, 1.0),
            AutomationTarget::EqBandParam(_, _, param) => match param {
                0 => (20.0, 20000.0),  // freq
                1 => (-24.0, 24.0),    // gain
                _ => (0.1, 10.0),      // Q
            },
            AutomationTarget::TrackSwing(_) => (0.0, 1.0),
            AutomationTarget::TrackHumanizeVelocity(_) => (0.0, 1.0),
            AutomationTarget::TrackHumanizeTiming(_) => (0.0, 1.0),
            AutomationTarget::TrackTimingOffset(_) => (-50.0, 50.0),
        }
    }
}

/// An automation lane containing points for a single parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationLane {
    pub id: AutomationLaneId,
    pub target: AutomationTarget,
    pub points: Vec<AutomationPoint>,
    pub enabled: bool,
    /// Whether this lane is armed for recording
    pub record_armed: bool,
    /// Minimum value for this parameter
    pub min_value: f32,
    /// Maximum value for this parameter
    pub max_value: f32,
}

impl AutomationLane {
    pub fn new(id: AutomationLaneId, target: AutomationTarget) -> Self {
        let (min_value, max_value) = target.default_range();
        Self {
            id,
            target,
            points: Vec::new(),
            enabled: true,
            record_armed: false,
            min_value,
            max_value,
        }
    }

    /// Add a point at the given tick (inserts in sorted order)
    pub fn add_point(&mut self, tick: u32, value: f32) {
        // Remove existing point at same tick
        self.points.retain(|p| p.tick != tick);

        let point = AutomationPoint::new(tick, value);
        let pos = self.points.iter().position(|p| p.tick > tick).unwrap_or(self.points.len());
        self.points.insert(pos, point);
    }

    /// Remove point at or near the given tick
    pub fn remove_point(&mut self, tick: u32) {
        self.points.retain(|p| p.tick != tick);
    }

    /// Get the interpolated value at a given tick position
    pub fn value_at(&self, tick: u32) -> Option<f32> {
        if self.points.is_empty() || !self.enabled {
            return None;
        }

        // Find surrounding points
        let mut prev: Option<&AutomationPoint> = None;
        let mut next: Option<&AutomationPoint> = None;

        for point in &self.points {
            if point.tick <= tick {
                prev = Some(point);
            } else {
                next = Some(point);
                break;
            }
        }

        let normalized = match (prev, next) {
            (Some(p), None) => p.value,
            (None, Some(n)) => n.value,
            (Some(p), Some(_n)) if p.tick == tick => p.value,
            (Some(p), Some(n)) => {
                // Interpolate between p and n
                let t = (tick - p.tick) as f32 / (n.tick - p.tick) as f32;
                Self::interpolate(p.value, n.value, t, p.curve)
            }
            (None, None) => return None,
        };

        // Convert from normalized (0-1) to actual value range
        Some(self.min_value + normalized * (self.max_value - self.min_value))
    }

    /// Interpolate between two values based on curve type
    fn interpolate(from: f32, to: f32, t: f32, curve: CurveType) -> f32 {
        match curve {
            CurveType::Linear => from + (to - from) * t,
            CurveType::Step => from,
            CurveType::Exponential => {
                // Exponential interpolation (good for frequency)
                let t_exp = t * t;
                from + (to - from) * t_exp
            }
            CurveType::SCurve => {
                // Smoothstep S-curve
                let t_smooth = t * t * (3.0 - 2.0 * t);
                from + (to - from) * t_smooth
            }
        }
    }

    /// Get the first point at or after the given tick
    pub fn point_at_or_after(&self, tick: u32) -> Option<&AutomationPoint> {
        self.points.iter().find(|p| p.tick >= tick)
    }

    /// Get the last point before the given tick
    pub fn point_before(&self, tick: u32) -> Option<&AutomationPoint> {
        self.points.iter().rev().find(|p| p.tick < tick)
    }

    /// Find point at exact tick
    pub fn point_at(&self, tick: u32) -> Option<&AutomationPoint> {
        self.points.iter().find(|p| p.tick == tick)
    }

    /// Find mutable point at exact tick
    pub fn point_at_mut(&mut self, tick: u32) -> Option<&mut AutomationPoint> {
        self.points.iter_mut().find(|p| p.tick == tick)
    }
}

/// Collection of automation lanes for a session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutomationState {
    pub lanes: Vec<AutomationLane>,
    pub selected_lane: Option<usize>,
    /// Next lane ID to assign (internal, but exposed for persistence)
    pub next_lane_id: AutomationLaneId,
}

impl AutomationState {
    pub fn new() -> Self {
        Self {
            lanes: Vec::new(),
            selected_lane: None,
            next_lane_id: 0,
        }
    }

    /// Recalculate next_lane_id from existing lanes (used after loading from DB)
    pub fn recalculate_next_lane_id(&mut self) {
        self.next_lane_id = self.lanes.iter().map(|l| l.id).max().map_or(0, |m| m + 1);
    }

    /// Add a new automation lane for a target
    pub fn add_lane(&mut self, target: AutomationTarget) -> AutomationLaneId {
        // Check if lane already exists for this target
        if let Some(existing) = self.lanes.iter().find(|l| l.target == target) {
            return existing.id;
        }

        let id = self.next_lane_id;
        self.next_lane_id += 1;
        let lane = AutomationLane::new(id, target);
        self.lanes.push(lane);

        if self.selected_lane.is_none() {
            self.selected_lane = Some(self.lanes.len() - 1);
        }

        id
    }

    /// Remove a lane by ID
    pub fn remove_lane(&mut self, id: AutomationLaneId) {
        if let Some(pos) = self.lanes.iter().position(|l| l.id == id) {
            self.lanes.remove(pos);
            // Adjust selection
            if let Some(sel) = self.selected_lane {
                if sel >= self.lanes.len() && !self.lanes.is_empty() {
                    self.selected_lane = Some(self.lanes.len() - 1);
                } else if self.lanes.is_empty() {
                    self.selected_lane = None;
                }
            }
        }
    }

    /// Get lane by ID
    pub fn lane(&self, id: AutomationLaneId) -> Option<&AutomationLane> {
        self.lanes.iter().find(|l| l.id == id)
    }

    /// Get mutable lane by ID
    pub fn lane_mut(&mut self, id: AutomationLaneId) -> Option<&mut AutomationLane> {
        self.lanes.iter_mut().find(|l| l.id == id)
    }

    /// Get lane for a specific target
    pub fn lane_for_target(&self, target: &AutomationTarget) -> Option<&AutomationLane> {
        self.lanes.iter().find(|l| &l.target == target)
    }

    /// Get mutable lane for a specific target
    pub fn lane_for_target_mut(&mut self, target: &AutomationTarget) -> Option<&mut AutomationLane> {
        self.lanes.iter_mut().find(|l| &l.target == target)
    }

    /// Get all lanes for a specific instrument
    pub fn lanes_for_instrument(&self, instrument_id: InstrumentId) -> Vec<&AutomationLane> {
        self.lanes.iter().filter(|l| l.target.instrument_id() == Some(instrument_id)).collect()
    }

    /// Selected lane
    pub fn selected(&self) -> Option<&AutomationLane> {
        self.selected_lane.and_then(|i| self.lanes.get(i))
    }

    /// Selected lane (mutable)
    pub fn selected_mut(&mut self) -> Option<&mut AutomationLane> {
        self.selected_lane.and_then(|i| self.lanes.get_mut(i))
    }

    /// Select next lane
    pub fn select_next(&mut self) {
        if self.lanes.is_empty() {
            self.selected_lane = None;
            return;
        }
        self.selected_lane = match self.selected_lane {
            None => Some(0),
            Some(i) if i + 1 < self.lanes.len() => Some(i + 1),
            Some(i) => Some(i),
        };
    }

    /// Select previous lane
    pub fn select_prev(&mut self) {
        if self.lanes.is_empty() {
            self.selected_lane = None;
            return;
        }
        self.selected_lane = match self.selected_lane {
            None => Some(0),
            Some(0) => Some(0),
            Some(i) => Some(i - 1),
        };
    }

    /// Remove all lanes for an instrument (when instrument is deleted)
    pub fn remove_lanes_for_instrument(&mut self, instrument_id: InstrumentId) {
        self.lanes.retain(|l| l.target.instrument_id() != Some(instrument_id));
        // Adjust selection
        if let Some(sel) = self.selected_lane {
            if sel >= self.lanes.len() {
                self.selected_lane = if self.lanes.is_empty() {
                    None
                } else {
                    Some(self.lanes.len() - 1)
                };
            }
        }
    }

    /// Remove all lanes for a bus (when bus is deleted)
    pub fn remove_lanes_for_bus(&mut self, bus_id: u8) {
        self.lanes.retain(|l| !matches!(l.target, AutomationTarget::BusLevel(id) if id == bus_id));
        // Adjust selection
        if let Some(sel) = self.selected_lane {
            if sel >= self.lanes.len() {
                self.selected_lane = if self.lanes.is_empty() {
                    None
                } else {
                    Some(self.lanes.len() - 1)
                };
            }
        }
    }
}
