# Groove Panel

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

- `TrackSwing`, `TrackHumanizeVelocity`, `TrackHumanizeTiming` action variants added
- `AutomationTarget` variants added for groove parameters
- Dispatch handlers need completion (see compiler errors in `automation.rs`, `mod.rs`)
