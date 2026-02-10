//! Automation lane types for parameter automation.

use serde::{Deserialize, Serialize};

use crate::{BusId, EffectId, InstrumentId, ParamIndex, ParameterTarget};

/// Whether target uses continuous interpolation or discrete steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueKind {
    /// 0.0-1.0 with interpolation
    Continuous,
    /// Step behavior, no interpolation
    Discrete,
}

impl Default for ValueKind {
    fn default() -> Self {
        Self::Continuous
    }
}

/// Discrete value representation for non-continuous automation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiscreteValue {
    Bool(bool),
    EnumIndex(u8),
    TimeSignature(u8, u8),
}

/// Kind of discrete value (for UI display).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscreteValueKind {
    Bool,
    EnumIndex,
    TimeSignature,
}

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

// ============================================================================
// New Structured AutomationTarget
// ============================================================================

/// Per-instrument parameter automation target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstrumentParameter {
    /// Standard modulatable parameter (reuses ParameterTarget)
    Standard(ParameterTarget),
}

/// Bus parameter target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BusParameter {
    /// Bus output level (0.0-1.0)
    Level,
}

/// Global session parameter target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GlobalParameter {
    /// Global BPM (30.0-300.0)
    Bpm,
    /// Global time signature (discrete)
    TimeSignature,
}

/// What parameter is being automated.
///
/// Structured enum that reuses ParameterTarget for per-instrument parameters,
/// providing a single source of truth for modulatable parameters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AutomationTarget {
    /// Per-instrument parameter automation
    Instrument(InstrumentId, InstrumentParameter),
    /// Bus output level
    Bus(BusId, BusParameter),
    /// Global session parameters
    Global(GlobalParameter),
}

impl AutomationTarget {
    // ========================================================================
    // Convenience constructors
    // ========================================================================

    /// Create an instrument target with a standard parameter.
    #[inline]
    pub fn instrument(id: InstrumentId, param: ParameterTarget) -> Self {
        Self::Instrument(id, InstrumentParameter::Standard(param))
    }

    /// Create a bus level target.
    #[inline]
    pub fn bus_level(bus_id: BusId) -> Self {
        Self::Bus(bus_id, BusParameter::Level)
    }

    /// Create a global BPM target.
    #[inline]
    pub fn bpm() -> Self {
        Self::Global(GlobalParameter::Bpm)
    }

    /// Create a global time signature target.
    #[inline]
    pub fn time_signature() -> Self {
        Self::Global(GlobalParameter::TimeSignature)
    }

    // ========================================================================
    // Common parameter shortcuts
    // ========================================================================

    #[inline]
    pub fn level(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::Level)
    }

    #[inline]
    pub fn pan(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::Pan)
    }

    #[inline]
    pub fn filter_cutoff(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::FilterCutoff)
    }

    #[inline]
    pub fn filter_resonance(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::FilterResonance)
    }

    #[inline]
    pub fn filter_bypass(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::FilterBypass)
    }

    #[inline]
    pub fn attack(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::Attack)
    }

    #[inline]
    pub fn decay(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::Decay)
    }

    #[inline]
    pub fn sustain(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::Sustain)
    }

    #[inline]
    pub fn release(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::Release)
    }

    #[inline]
    pub fn lfo_rate(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::LfoRate)
    }

    #[inline]
    pub fn lfo_depth(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::LfoDepth)
    }

    #[inline]
    pub fn send_level(id: InstrumentId, bus_id: BusId) -> Self {
        Self::instrument(id, ParameterTarget::SendLevel(bus_id))
    }

    #[inline]
    pub fn sample_rate(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::SampleRate)
    }

    #[inline]
    pub fn sample_amp(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::SampleAmp)
    }

    #[inline]
    pub fn vst_param(id: InstrumentId, param_idx: u32) -> Self {
        Self::instrument(id, ParameterTarget::VstParam(param_idx))
    }

    #[inline]
    pub fn effect_param(id: InstrumentId, effect_id: EffectId, param_idx: ParamIndex) -> Self {
        Self::instrument(id, ParameterTarget::EffectParam(effect_id, param_idx))
    }

    #[inline]
    pub fn effect_bypass(id: InstrumentId, effect_id: EffectId) -> Self {
        Self::instrument(id, ParameterTarget::EffectBypass(effect_id))
    }

    #[inline]
    pub fn eq_band_freq(id: InstrumentId, band: usize) -> Self {
        Self::instrument(id, ParameterTarget::EqBandFreq(band))
    }

    #[inline]
    pub fn eq_band_gain(id: InstrumentId, band: usize) -> Self {
        Self::instrument(id, ParameterTarget::EqBandGain(band))
    }

    #[inline]
    pub fn eq_band_q(id: InstrumentId, band: usize) -> Self {
        Self::instrument(id, ParameterTarget::EqBandQ(band))
    }

    #[inline]
    pub fn swing(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::Swing)
    }

    #[inline]
    pub fn humanize_velocity(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::HumanizeVelocity)
    }

    #[inline]
    pub fn humanize_timing(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::HumanizeTiming)
    }

    #[inline]
    pub fn timing_offset(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::TimingOffset)
    }

    #[inline]
    pub fn track_time_signature(id: InstrumentId) -> Self {
        Self::instrument(id, ParameterTarget::TimeSignature)
    }

    // ========================================================================
    // Query methods
    // ========================================================================

    /// Get the instrument ID associated with this target (None for global/bus targets).
    pub fn instrument_id(&self) -> Option<InstrumentId> {
        match self {
            AutomationTarget::Instrument(id, _) => Some(*id),
            AutomationTarget::Bus(_, _) | AutomationTarget::Global(_) => None,
        }
    }

    /// Get the underlying ParameterTarget if this is an instrument target with a standard param.
    pub fn parameter_target(&self) -> Option<&ParameterTarget> {
        match self {
            AutomationTarget::Instrument(_, InstrumentParameter::Standard(param)) => Some(param),
            _ => None,
        }
    }

    /// Get a human-readable name for this target.
    pub fn name(&self) -> String {
        match self {
            AutomationTarget::Instrument(_, InstrumentParameter::Standard(param)) => param.name(),
            AutomationTarget::Bus(bus_id, BusParameter::Level) => format!("Bus {} Level", bus_id),
            AutomationTarget::Global(GlobalParameter::Bpm) => "BPM".to_string(),
            AutomationTarget::Global(GlobalParameter::TimeSignature) => "Time Signature".to_string(),
        }
    }

    /// Get a short name for compact display.
    pub fn short_name(&self) -> &'static str {
        match self {
            AutomationTarget::Instrument(_, InstrumentParameter::Standard(param)) => param.short_name(),
            AutomationTarget::Bus(_, BusParameter::Level) => "BusLv",
            AutomationTarget::Global(GlobalParameter::Bpm) => "BPM",
            AutomationTarget::Global(GlobalParameter::TimeSignature) => "TSig",
        }
    }

    /// Get all possible automation targets for an instrument (static set).
    pub fn targets_for_instrument(id: InstrumentId) -> Vec<AutomationTarget> {
        vec![
            Self::level(id),
            Self::pan(id),
            Self::filter_cutoff(id),
            Self::filter_resonance(id),
            Self::filter_bypass(id),
            Self::lfo_rate(id),
            Self::lfo_depth(id),
            Self::attack(id),
            Self::decay(id),
            Self::sustain(id),
            Self::release(id),
            Self::swing(id),
            Self::humanize_velocity(id),
            Self::humanize_timing(id),
            Self::timing_offset(id),
            Self::track_time_signature(id),
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
            AutomationTarget::Instrument(_, InstrumentParameter::Standard(param)) => param.default_range(),
            AutomationTarget::Bus(_, BusParameter::Level) => (0.0, 1.0),
            AutomationTarget::Global(GlobalParameter::Bpm) => (30.0, 300.0),
            AutomationTarget::Global(GlobalParameter::TimeSignature) => (0.0, 1.0),
        }
    }

    /// Get the value kind for this target (continuous or discrete).
    pub fn value_kind(&self) -> ValueKind {
        match self {
            AutomationTarget::Instrument(_, InstrumentParameter::Standard(param)) => {
                match param {
                    ParameterTarget::FilterBypass
                    | ParameterTarget::EffectBypass(_)
                    | ParameterTarget::TimeSignature => ValueKind::Discrete,
                    _ => ValueKind::Continuous,
                }
            }
            AutomationTarget::Bus(_, _) => ValueKind::Continuous,
            AutomationTarget::Global(GlobalParameter::Bpm) => ValueKind::Continuous,
            AutomationTarget::Global(GlobalParameter::TimeSignature) => ValueKind::Discrete,
        }
    }

    /// Get the discrete value kind for this target (if discrete).
    pub fn discrete_value_kind(&self) -> Option<DiscreteValueKind> {
        match self {
            AutomationTarget::Global(GlobalParameter::TimeSignature) => {
                Some(DiscreteValueKind::TimeSignature)
            }
            AutomationTarget::Instrument(_, InstrumentParameter::Standard(param)) => {
                match param {
                    ParameterTarget::TimeSignature => Some(DiscreteValueKind::TimeSignature),
                    ParameterTarget::FilterBypass | ParameterTarget::EffectBypass(_) => {
                        Some(DiscreteValueKind::Bool)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Get the available options for discrete targets.
    /// Returns None for continuous targets.
    pub fn discrete_options(&self) -> Option<Vec<String>> {
        match self.discrete_value_kind()? {
            DiscreteValueKind::Bool => Some(vec!["Off".to_string(), "On".to_string()]),
            DiscreteValueKind::TimeSignature => Some(vec![
                "4/4".to_string(),
                "3/4".to_string(),
                "5/4".to_string(),
                "6/8".to_string(),
                "7/8".to_string(),
                "12/8".to_string(),
            ]),
            DiscreteValueKind::EnumIndex => None,
        }
    }

    /// Convert a normalized 0.0-1.0 value to a discrete value.
    pub fn normalized_to_discrete(&self, normalized: f32) -> Option<DiscreteValue> {
        match self.discrete_value_kind()? {
            DiscreteValueKind::Bool => Some(DiscreteValue::Bool(normalized >= 0.5)),
            DiscreteValueKind::EnumIndex => {
                let options = self.discrete_options()?;
                let index = ((normalized * options.len() as f32) as usize).min(options.len() - 1);
                Some(DiscreteValue::EnumIndex(index as u8))
            }
            DiscreteValueKind::TimeSignature => {
                // Map normalized value to time signature options
                let signatures: [(u8, u8); 6] = [(4, 4), (3, 4), (5, 4), (6, 8), (7, 8), (12, 8)];
                let index = ((normalized * signatures.len() as f32) as usize).min(signatures.len() - 1);
                let (num, denom) = signatures[index];
                Some(DiscreteValue::TimeSignature(num, denom))
            }
        }
    }

    /// Convert a discrete value to normalized 0.0-1.0.
    pub fn discrete_to_normalized(&self, discrete: &DiscreteValue) -> Option<f32> {
        match (self.discrete_value_kind()?, discrete) {
            (DiscreteValueKind::Bool, DiscreteValue::Bool(b)) => Some(if *b { 1.0 } else { 0.0 }),
            (DiscreteValueKind::EnumIndex, DiscreteValue::EnumIndex(idx)) => {
                let options = self.discrete_options()?;
                Some(*idx as f32 / (options.len() - 1).max(1) as f32)
            }
            (DiscreteValueKind::TimeSignature, DiscreteValue::TimeSignature(num, denom)) => {
                let signatures: [(u8, u8); 6] = [(4, 4), (3, 4), (5, 4), (6, 8), (7, 8), (12, 8)];
                let index = signatures.iter().position(|&(n, d)| n == *num && d == *denom)?;
                Some(index as f32 / (signatures.len() - 1).max(1) as f32)
            }
            _ => None,
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

        // Binary search: find first point with tick > target
        // partition_point returns the index where the predicate becomes false
        // i.e., the first index where p.tick > tick
        let next_idx = self.points.partition_point(|p| p.tick <= tick);

        let prev = if next_idx > 0 {
            Some(&self.points[next_idx - 1])
        } else {
            None
        };
        let next = self.points.get(next_idx);

        // Force step behavior for discrete targets
        let force_step = self.target.value_kind() == ValueKind::Discrete;

        let normalized = match (prev, next) {
            (Some(p), None) => p.value,
            (None, Some(n)) => n.value,
            (Some(p), Some(_n)) if p.tick == tick => p.value,
            (Some(p), Some(n)) => {
                if force_step || p.curve == CurveType::Step {
                    // Discrete targets or step curve: hold until next point
                    p.value
                } else {
                    // Interpolate between p and n
                    let t = (tick - p.tick) as f32 / (n.tick - p.tick) as f32;
                    Self::interpolate(p.value, n.value, t, p.curve)
                }
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
    pub fn remove_lanes_for_bus(&mut self, bus_id: BusId) {
        self.lanes.retain(|l| !matches!(l.target, AutomationTarget::Bus(id, _) if id == bus_id));
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
