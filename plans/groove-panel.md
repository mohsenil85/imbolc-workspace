# Groove Panel

**Status:** IN_PROGRESS
**Last Updated:** 2025-02-06

## Concept

"Groove" as umbrella term for all timing/feel controls on a track.

## Controls

| Setting | Type | Description |
|---------|------|-------------|
| Quantize | Grid + Strength % | How strictly notes snap to grid |
| Swing | % + Grid | Delay off-beats (50%=straight, 66%=triplet) |
| Humanize Timing | % | Random micro-timing variation |
| Humanize Velocity | % | Random velocity variation |
| Push/Pull | ms | Consistent timing offset (drag/rush) |

## Terminology

- **Swing** is a specific, quantifiable timing shift — delays every other note by a percentage
- **Groove** is the broader concept encompassing swing, velocity variations, and timing humanization
- **Push/Pull** (aka drag/rush) is a consistent offset — playing ahead or behind the beat

## UI Layout

```
┌─ Groove ─────────────────────┐
│ Quantize:  1/16  Strength 80%│
│ Swing:     62%   Grid 1/8    │
│ Humanize:  Timing 12% Vel 8% │
│ Push/Pull: -5ms              │
└──────────────────────────────┘
```

Quantize at top (grid adherence), expressive deviations below.

## Design Notes

- All settings non-destructive (applied at playback)
- Per-track settings with optional global defaults
- Swing grid typically independent of quantize grid (e.g., quantize to 1/16, swing on 1/8)

## Implementation Status

| Component | Status |
|-----------|--------|
| Action Variants | ✓ Complete |
| AutomationTarget Variants | ✓ Complete |
| Dispatch Handlers | ✓ Complete |
| Track State (GrooveConfig) | ✓ Complete |
| Global Settings | ✓ Complete |
| UI Panel | Not started |
| Quantize Feature | Not started |

## Next Steps

### 1. Create Groove Pane UI
- [ ] Create `imbolc-ui/src/panes/groove_pane.rs`
- [ ] Register pane in `imbolc-ui/src/main.rs`
- [ ] Add keybinding to open groove panel (suggest: `g` from track context)
- [ ] Implement rendering per UI Layout mockup
- [ ] Wire controls to existing actions:
  - `SetTrackSwing` / `AdjustTrackSwing`
  - `SetTrackSwingGrid`
  - `SetTrackHumanizeVelocity` / `AdjustTrackHumanizeVelocity`
  - `SetTrackHumanizeTiming` / `AdjustTrackHumanizeTiming`
  - `SetTrackTimingOffset` / `AdjustTrackTimingOffset`
  - `ResetTrackGroove`

### 2. Implement Quantize Feature
- [ ] Add `QuantizeConfig` to `GrooveConfig` (grid + strength %)
- [ ] Add `SetTrackQuantize`, `SetTrackQuantizeStrength` actions
- [ ] Add dispatch handlers
- [ ] Wire to UI

### 3. Audio Integration
- [ ] Verify groove parameters affect note scheduling in audio engine
- [ ] Test swing/humanize/timing-offset at playback time
