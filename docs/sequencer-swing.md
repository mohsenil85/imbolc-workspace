# Sequencer & Swing

## Overview

Step sequencer with advanced swing/groove controls for humanized, musical timing.

## Sequencer Basics

```
Track 0 (Kick):   [X . . . | X . . . | X . . . | X . . . ]
Track 1 (Snare):  [. . X . | . . X . | . . X . | . . X . ]
Track 2 (HiHat):  [X . X . | X . X . | X . X . | X . X . ]
Track 3 (Bass):   [X . . X | . . X . | X . . X | . . X . ]
                   1 2 3 4   5 6 7 8   9 ...
```

### Parameters
- **BPM**: 20-300 (default 120)
- **Steps**: 16, 32, or 64
- **Step division**: 1/16th (default), 1/8th, 1/32nd

### Per-Step Data
- **Velocity**: 0.0-1.0 (dynamics)
- **Pitch**: MIDI note or offset from base
- **Gate length**: % of step duration
- **Probability**: % chance step triggers

## Swing System

Swing delays off-beat notes to create groove. This implementation goes beyond basic swing.

### Global Swing

```
Amount: 50% (straight) ──────────────── 75% (heavy triplet)
        │                                │
        ▼                                ▼
    [X   X   X   X ]              [X    X  X    X ]
     1   2   3   4                 1    2  3    4
```

**Amount** (0-100%):
- 50% = straight timing (no swing)
- 66% = triplet feel (classic swing)
- 75% = heavy, laid-back groove

### Swing Grid

Apply swing to different subdivisions:

| Grid | Affects | Feel |
|------|---------|------|
| 8ths | beats 2, 4, 6, 8... | Basic shuffle |
| 16ths | off-16ths (e, a) | Tighter groove |
| Both | 8ths and 16ths | Complex shuffle |

```
16th grid:  1 e & a 2 e & a 3 e & a 4 e & a
Swing 16:   X . X . X . X . (delays the 'e' and 'a')
Swing 8:    X . . . X . . . (delays beats 2, 4)
```

### Per-Track Swing

Different tracks can have different swing amounts:

```java
record TrackSwing(
    double amount,      // 0.0-1.0 (0.5 = straight)
    SwingGrid grid,     // EIGHTHS, SIXTEENTHS, BOTH
    boolean enabled     // override global
) {}
```

Example: Kick straight, hi-hats swung
```
Kick (0% swing):    [X . . . X . . . X . . . X . . . ]
HiHat (66% swing):  [X .  X. X .  X. X .  X. X .  X. ]
```

### Humanize

Random timing variation for natural feel:

```java
record Humanize(
    double timing,      // ±ms random offset (0-50ms)
    double velocity,    // ±% velocity variation
    double gateLength   // ±% gate variation
) {}
```

### Groove Templates

Pre-defined swing patterns:

| Template | Swing | Grid | Character |
|----------|-------|------|-----------|
| Straight | 50% | - | Mechanical, precise |
| Light Shuffle | 58% | 8ths | Subtle movement |
| MPC Swing | 66% | 16ths | Classic hip-hop |
| Triplet | 66% | 8ths | Jazz/blues feel |
| Heavy | 72% | 16ths | Laid back, loose |
| Drunk | 54% + humanize | 16ths | Slightly off, human |

## State Structure

```java
record Sequencer(
    int bpm,
    int steps,
    int currentStep,
    boolean playing,
    List<Track> tracks,
    SwingSettings swing
) {}

record Track(
    String targetModule,
    String targetParam,
    List<Step> steps,
    TrackSwing swing,        // per-track override
    Humanize humanize
) {}

record Step(
    boolean active,
    double velocity,
    int pitch,
    double gateLength,
    double probability
) {}

record SwingSettings(
    double amount,           // global swing 0.0-1.0
    SwingGrid grid,          // EIGHTHS, SIXTEENTHS, BOTH
    String template          // preset name or "custom"
) {}

enum SwingGrid { EIGHTHS, SIXTEENTHS, BOTH }
```

## TUI Controls

### Sequencer View (`s` from rack)

```
┌─SEQUENCER──────────────────────────────────────────────┐
│ BPM: 120    Steps: 16    Swing: 66% (MPC)             │
│                                                        │
│         1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6               │
│ Kick    X . . . X . . . X . . . X . . .  [saw-1:gate] │
│ Snare   . . . . X . . . . . . . X . . .  [noise:gate] │
│>HiHat   X . X . X . X . X . X . X . X .  [saw-2:gate] │
│ Bass    X . . X . . X . X . . X . . X .  [saw-3:freq] │
│                     ▲                                  │
│                  cursor                                │
├────────────────────────────────────────────────────────┤
│ Step 7: vel=0.8  pitch=+0  gate=80%  prob=100%        │
│ Track swing: 66% (global)  humanize: off              │
└────────────────────────────────────────────────────────┘
[j/k] Track  [h/l] Step  [Enter] Toggle  [+/-] Velocity
[S] Swing settings  [H] Humanize  [Space] Play/Stop
```

### Swing Settings View

```
┌─SWING SETTINGS─────────────────────────────────────────┐
│                                                        │
│ Global Swing                                           │
│ ─────────────                                          │
│ Amount:   [====████████========] 66%                   │
│ Grid:     ( ) 8ths  (•) 16ths  ( ) Both               │
│ Template: [MPC Swing         ▼]                        │
│                                                        │
│ Per-Track Overrides                                    │
│ ───────────────────                                    │
│ Kick:    [global]                                      │
│ Snare:   [global]                                      │
│ HiHat:   [72% / 16ths]  ← custom                       │
│ Bass:    [global]                                      │
│                                                        │
│ Humanize                                               │
│ ────────                                               │
│ Timing:  ±12ms                                         │
│ Velocity: ±10%                                         │
│                                                        │
└────────────────────────────────────────────────────────┘
[j/k] Select  [h/l] Adjust  [Enter] Edit track  [Esc] Back
```

## Implementation Notes

### Swing Calculation

```java
/**
 * Calculate actual trigger time for a step with swing applied.
 * @param stepIndex 0-based step number
 * @param stepDuration duration of one step in ms
 * @param swingAmount 0.0-1.0 where 0.5 is straight
 * @param grid which subdivisions to swing
 * @return offset in ms from grid time
 */
double calculateSwingOffset(int stepIndex, double stepDuration,
                            double swingAmount, SwingGrid grid) {
    boolean isSwungStep = switch (grid) {
        case EIGHTHS -> (stepIndex % 2) == 1;      // 2, 4, 6, 8...
        case SIXTEENTHS -> (stepIndex % 2) == 1;   // odd steps
        case BOTH -> (stepIndex % 2) == 1;         // same for now
    };

    if (!isSwungStep) return 0;

    // Convert 0.5-1.0 range to 0-0.5 step delay
    double delay = (swingAmount - 0.5) * stepDuration;
    return delay;
}
```

### Clock Implementation

```java
class SequencerClock {
    private ScheduledExecutorService scheduler;
    private long stepDurationMicros;

    void start(int bpm, Consumer<Integer> onStep) {
        stepDurationMicros = (60_000_000L / bpm) / 4;  // 16th notes

        scheduler.scheduleAtFixedRate(() -> {
            int step = currentStep.getAndIncrement() % totalSteps;

            // Calculate swing offset for this step
            long swingOffsetMicros = calculateSwingOffset(step, ...);

            if (swingOffsetMicros > 0) {
                // Delay this step's triggers
                scheduler.schedule(() -> onStep.accept(step),
                                   swingOffsetMicros, MICROSECONDS);
            } else {
                onStep.accept(step);
            }
        }, 0, stepDurationMicros, MICROSECONDS);
    }
}
```

## AI Integration

Haiku can generate patterns:

**User**: "make a four on the floor beat"
**Haiku returns**:
```json
{
  "actions": [
    {"type": "set_pattern", "track": 0, "name": "Kick",
     "steps": [1,0,0,0, 1,0,0,0, 1,0,0,0, 1,0,0,0]},
    {"type": "set_pattern", "track": 1, "name": "Snare",
     "steps": [0,0,0,0, 1,0,0,0, 0,0,0,0, 1,0,0,0]},
    {"type": "set_pattern", "track": 2, "name": "HiHat",
     "steps": [1,0,1,0, 1,0,1,0, 1,0,1,0, 1,0,1,0]}
  ]
}
```

**User**: "add some swing to make it groovier"
**Haiku returns**:
```json
{
  "actions": [
    {"type": "set_swing", "amount": 0.62, "grid": "16ths"},
    {"type": "set_humanize", "timing": 8}
  ],
  "explanation": "Added 62% swing on 16ths with slight timing humanization"
}
```

**User**: "make the hi-hats more swung than the kick"
**Haiku returns**:
```json
{
  "actions": [
    {"type": "set_track_swing", "track": 0, "amount": 0.5},
    {"type": "set_track_swing", "track": 2, "amount": 0.70}
  ],
  "explanation": "Kick is now straight, hi-hats have heavy 70% swing"
}
```
