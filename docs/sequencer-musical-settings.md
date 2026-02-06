# Sequencer Musical Settings

> **Note:** This document uses Java/record syntax from an earlier prototype.
> Concepts remain valid; see `imbolc-types/src/state/` for Rust implementations.

## Overview

The sequencer view includes musical context settings that affect note display, input behavior, and playback. Settings are global by default but can be overridden per-track.

## Settings

### Key (Root + Scale)

**Root Notes:** C, C#, D, D#, E, F, F#, G, G#, A, A#, B

**Scale Types:**
| Scale | Pattern (semitones) | Notes in C | Use Case |
|-------|---------------------|------------|----------|
| Major | 2-2-1-2-2-2-1 | C D E F G A B | Happy, bright |
| Minor (Natural) | 2-1-2-2-1-2-2 | C D Eb F G Ab Bb | Sad, dark |
| Dorian | 2-1-2-2-2-1-2 | C D Eb F G A Bb | Jazz, funk |
| Mixolydian | 2-2-1-2-2-1-2 | C D E F G A Bb | Blues rock |
| Pentatonic Major | 2-2-3-2-3 | C D E G A | Simple, universal |
| Pentatonic Minor | 3-2-2-3-2 | C Eb F G Bb | Blues, rock |
| Blues | 3-2-1-1-3-2 | C Eb F F# G Bb | Blues |
| Harmonic Minor | 2-1-2-2-1-3-1 | C D Eb F G Ab B | Classical, metal |
| Melodic Minor | 2-1-2-2-2-2-1 | C D Eb F G A B | Jazz |
| Chromatic | 1-1-1-1-1-1-1-1-1-1-1-1 | All 12 notes | No restriction |

### Time Signature (Meter)

Common meters:
- **4/4** - Standard "common time"
- **3/4** - Waltz
- **6/8** - Compound duple (feels like 2 groups of 3)
- **2/4** - March, polka
- **5/4** - Odd meter (Take Five)
- **7/8** - Odd meter (progressive)
- **12/8** - Compound quadruple (blues shuffle)

Affects:
- Bar line positions in grid
- Beat emphasis (downbeat highlighting)
- Metronome accent pattern
- Step grouping display

### Tempo

- Range: 20-300 BPM
- Resolution: 0.1 BPM
- Tap tempo support (future)

### Grid Resolution & Zoom

The grid determines the smallest time division visible and editable. Zoom controls how much musical time fits on screen.

**Grid Divisions (finest to coarsest):**
| Division | Name | Per Quarter Note | Use Case |
|----------|------|------------------|----------|
| 1/64 | Sixty-fourth | 16 | Extreme detail, rolls |
| 1/32 | Thirty-second | 8 | Fast runs, ornaments |
| 1/16 | Sixteenth | 4 | Common default, hi-hats |
| 1/8 | Eighth | 2 | Melodies, bass lines |
| 1/4 | Quarter | 1 | Basic beats, chords |

**Triplet Divisions:**
| Division | Name | Per Quarter Note | Use Case |
|----------|------|------------------|----------|
| 1/64T | Sixty-fourth triplet | 24 | Extreme shuffle |
| 1/32T | Thirty-second triplet | 12 | Fast triplet runs |
| 1/16T | Sixteenth triplet | 6 | Shuffle feel |
| 1/8T | Eighth triplet | 3 | Swing, jazz |
| 1/4T | Quarter triplet | 1.5 | Polyrhythm |

**Zoom Levels:**

Zoom controls how many bars are visible at once. Independent of grid resolution.

| Zoom | Bars Visible | Best For |
|------|--------------|----------|
| 1 (closest) | 1 bar | Detailed editing |
| 2 | 2 bars | Phrase editing |
| 4 | 4 bars | Section overview |
| 8 | 8 bars | Arrangement view |
| 16 | 16 bars | Full song overview |

**Relationship:**
- Grid = snap resolution (where notes can be placed)
- Zoom = visual scale (how much you see)
- You can have fine grid (1/64) at any zoom level
- Zoomed out + fine grid = notes may overlap visually

**Snap Behavior:**

1. **Snap to Grid ON (default)**
   - Note start times round to nearest grid line
   - Moving notes horizontally snaps to grid
   - Note lengths snap to grid divisions

2. **Snap to Grid OFF**
   - Free placement at any position
   - For humanization, rubato
   - Still stored at high resolution (1/64 or finer internally)

3. **Quantize Command**
   - Snap existing notes to current grid
   - Options: start only, end only, both
   - Strength: 0-100% (partial quantize for feel)

**Internal Resolution:**

Internally, time is stored at 1/64 note resolution minimum (PPQN = 64).
- PPQN = Pulses Per Quarter Note
- 64 PPQN allows 1/64 notes and 1/32 triplets (which need divisibility by 3×16=48, but we use 64 for clean binary)
- For triplets: 1/16T = 64/6 ≈ 10.67 ticks (we'll use 96 PPQN for perfect triplet support)

**Recommended: 96 PPQN**
- Divisible by 2, 3, 4, 6, 8, 12, 16, 24, 32, 48
- Supports all standard divisions and triplets
- 1/64 note = 1.5 ticks (round to 2 for display)
- 1/32T = 2 ticks (exact)

### Tuning Reference (A=)

- Default: 440 Hz
- Range: 400-480 Hz
- Common alternatives:
  - 432 Hz ("Verdi tuning")
  - 442 Hz (European orchestras)
  - 444 Hz (some orchestras)

Affects pitch-to-frequency conversion:
```
freq = tuningHz * 2^((midiNote - 69) / 12)
```

## Hierarchy: Global vs Per-Track

```
Global Settings (Project Level)
├── Key: Am
├── Meter: 4/4
├── Tempo: 120 BPM
└── Tuning: 440 Hz

Track 1: saw-1
├── Key Override: [none] → uses Am
└── Shows Am scale

Track 2: bass-1
├── Key Override: A Pentatonic Minor
└── Shows A Pentatonic Minor (subset of Am)

Track 3: lead-1
├── Key Override: A Dorian
└── Shows A Dorian
```

**Rules:**
- Tempo and Tuning are always global (no per-track override)
- Key can be overridden per-track
- Meter could be per-track in future (polyrhythm) but global for now

## UI Layout

```
╭─ SEQUENCER ─────────────────────────────────────────────────────────────────╮
│ ┌─ Settings ──────────────────────────────────────────────────────────────┐ │
│ │ Key: [Am]  Meter: [4/4]  Tempo: [120] BPM  Grid: [1/16]  Zoom: [2 bars] │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ Track: saw-1 [freq]                Key: Am (global)    Snap: [ON]          │
│ ═══════════════════════════════════════════════════════════════════════════ │
│ Bar 1               │ Bar 2               │  ← bar markers (zoom=2)         │
│      │1 . . . │2 . . . │3 . . . │4 . . . │1 . . . │2 . . . │3 . . . │4     │
│ ─────┼────────┼────────┼────────┼────────┼────────┼────────┼────────┼───── │
│  C5  │ ●      │        │ ●      │        │        │        │        │      │
│  B4  │        │        │        │        │        │        │        │      │
│  A4 ★│   ●    │   ●    │   ●    │   ●    │        │        │        │      │
│  G#4 │        │        │        │        │        │        │        │      │
│  G4  │        │ ●      │        │ ●      │        │        │        │      │
│  F#4 │        │        │        │        │        │        │        │      │
│  F4  │        │        │        │        │        │        │        │      │
│  E4  │ ●      │        │ ●      │        │        │        │        │      │
│ ═════╧════════╧════════╧════════╧════════╧════════╧════════╧════════╧═════ │
├─────────────────────────────────────────────────────────────────────────────┤
│ [g] Settings  [+/-] Zoom  [</>>] Grid  [S] Snap  [Space] Play  [q] Quantize │
╰─────────────────────────────────────────────────────────────────────────────╯
```

**Grid at different zoom levels:**

Zoom = 1 bar, Grid = 1/16:
```
│1   .   .   .   │2   .   .   .   │3   .   .   .   │4   .   .   .   │
```

Zoom = 4 bars, Grid = 1/16 (compressed):
```
│1234│1234│1234│1234│  ← beats shown as single chars
```

Zoom = 1 bar, Grid = 1/64 (maximum detail):
```
│1 . . . . . . . . . . . . . . . │2 . . . . . . . ...  ← 16 subdivisions per beat
```

### Visual Indicators

| Element | Display | Color |
|---------|---------|-------|
| In-key notes | Normal brightness | Default |
| Out-of-key notes | Dimmed | Gray |
| Root note | Highlighted, ★ marker | Cyan/Bold |
| Active steps | ● | Bright |
| Current playhead | Column highlight | Inverse |
| Bar lines | Thicker │ | Bold |
| Beat lines | Normal │ | Normal |
| Sub-beat | Thin . | Dim |

## Snap Behaviors

### Snap-to-Key (Pitch)

When entering/editing notes:

1. **Pitch navigation (↑/↓)** snaps to next in-key note
   - In Am: A → B → C → D → E → F → G → A (skips A#, C#, D#, F#, G#)
   - Holding Shift allows chromatic movement (all semitones)

2. **Quantize pitch** - snap existing notes to nearest in-key pitch
   - Out-of-key note moves to nearest scale degree
   - Example in Am: G# → G or A (whichever is closer)

3. **Chromatic scale** - no pitch snapping, all notes available

### Snap-to-Grid (Time)

When snap is ON:

1. **Horizontal navigation (←/→)** moves by grid divisions
   - At 1/16 grid: cursor jumps 1/16 note at a time
   - Holding Shift moves by beat (1/4 note)
   - Holding Ctrl moves by bar

2. **Note placement** snaps start time to grid
   - Click/enter places note at nearest grid line
   - Note length defaults to grid division (can be changed)

3. **Moving notes** snaps to grid
   - Dragging horizontally snaps to grid lines
   - Vertical (pitch) movement is independent

4. **Quantize time** - snap existing note timing to grid
   - Options: start only, end only, both, length
   - Strength: 100% (hard) to 0% (no change)
   - Partial quantize (e.g., 50%) moves halfway to grid

When snap is OFF:
- Free movement at tick resolution (1/96 of quarter note)
- For rubato, humanized timing
- Notes display at exact position

### Combined Quantize

The 'q' command quantizes based on current settings:
- If in pitch-snapping mode: quantize pitch to key
- If in time-snapping mode: quantize time to grid
- 'Q' (shift+q): quantize both pitch and time

## Keybindings

### Global Settings (press 'g' to enter settings mode)

| Key | Action |
|-----|--------|
| `k` / `K` | Cycle root note down/up (C → C# → D...) |
| `s` / `S` | Cycle scale type |
| `m` / `M` | Cycle meter presets |
| `t` / `T` | Adjust tempo -/+ 1 BPM |
| `Shift+t` | Adjust tempo -/+ 10 BPM |
| `a` / `A` | Adjust tuning -/+ 1 Hz |
| `Escape` | Exit settings mode |

### Grid & Zoom (in normal sequencer mode)

| Key | Action |
|-----|--------|
| `<` / `>` | Grid finer/coarser (1/64 ↔ 1/32 ↔ 1/16 ↔ 1/8 ↔ 1/4) |
| `Shift+<` / `Shift+>` | Grid to triplet variant (1/16 ↔ 1/16T) |
| `+` / `-` | Zoom in/out (fewer/more bars visible) |
| `S` | Toggle snap to grid ON/OFF |
| `0` | Reset zoom to default (2 bars) |

### Per-Track Key Override (press 'k' in normal mode)

| Key | Action |
|-----|--------|
| `k` | Open track key selector |
| `←/→` | Cycle root note |
| `↑/↓` | Cycle scale type |
| `Backspace` | Clear override (use global) |
| `Enter` | Confirm |
| `Escape` | Cancel |

### Note Entry & Navigation

| Key | Action |
|-----|--------|
| `←/→` | Move cursor by grid division (snap ON) or tick (snap OFF) |
| `Shift+←/→` | Move cursor by beat |
| `Ctrl+←/→` | Move cursor by bar |
| `↑/↓` | Move pitch (snaps to key if enabled) |
| `Shift+↑/↓` | Move pitch chromatically |
| `Home` / `End` | Jump to start/end of pattern |
| `PgUp` / `PgDn` | Move cursor by visible screen width |

### Quantize Commands

| Key | Action |
|-----|--------|
| `q` | Quantize: pitch to key (if key snap) or time to grid (if time snap) |
| `Q` | Quantize both pitch and time |
| `Ctrl+q` | Open quantize options (strength, what to quantize) |

## State Records

```java
// Root note of a key
enum Note {
    C, Cs, D, Ds, E, F, Fs, G, Gs, A, As, B;

    public int semitone() { return ordinal(); }
    public String displayName() {
        return switch (this) {
            case Cs -> "C#"; case Ds -> "D#"; case Fs -> "F#";
            case Gs -> "G#"; case As -> "A#";
            default -> name();
        };
    }
}

// Scale type with interval pattern
enum Scale {
    MAJOR(new int[]{2, 2, 1, 2, 2, 2, 1}),
    MINOR(new int[]{2, 1, 2, 2, 1, 2, 2}),
    DORIAN(new int[]{2, 1, 2, 2, 2, 1, 2}),
    MIXOLYDIAN(new int[]{2, 2, 1, 2, 2, 1, 2}),
    PENTATONIC_MAJOR(new int[]{2, 2, 3, 2, 3}),
    PENTATONIC_MINOR(new int[]{3, 2, 2, 3, 2}),
    BLUES(new int[]{3, 2, 1, 1, 3, 2}),
    HARMONIC_MINOR(new int[]{2, 1, 2, 2, 1, 3, 1}),
    MELODIC_MINOR(new int[]{2, 1, 2, 2, 2, 2, 1}),
    CHROMATIC(new int[]{1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1});

    private final int[] intervals;
    // methods...
}

// Musical key = root + scale
record MusicalKey(Note root, Scale scale) {
    public static MusicalKey of(Note root, Scale scale) {
        return new MusicalKey(root, scale);
    }

    public static MusicalKey Am() {
        return new MusicalKey(Note.A, Scale.MINOR);
    }

    // Get all MIDI notes in this key within a range
    public List<Integer> getScaleNotes(int lowMidi, int highMidi) { /* ... */ }

    // Check if a MIDI note is in this key
    public boolean contains(int midiNote) { /* ... */ }

    // Snap a MIDI note to nearest in-key note
    public int snap(int midiNote) { /* ... */ }

    // Get next in-key note above/below
    public int nextInKey(int midiNote, int direction) { /* ... */ }

    public String displayName() {
        return root.displayName() + " " + scale.name().toLowerCase().replace("_", " ");
    }
}

// Time signature
record TimeSignature(int numerator, int denominator) {
    public static TimeSignature FOUR_FOUR = new TimeSignature(4, 4);
    public static TimeSignature THREE_FOUR = new TimeSignature(3, 4);
    public static TimeSignature SIX_EIGHT = new TimeSignature(6, 8);
    // etc.

    public int beatsPerBar() { return numerator; }
    public int stepsPerBeat(int stepsPerQuarter) {
        // For 4/4 with 4 steps/quarter: 4 steps per beat
        // For 6/8 with 4 steps/quarter: 2 steps per beat (8th note = half quarter)
        return stepsPerQuarter * 4 / denominator;
    }

    public String displayName() { return numerator + "/" + denominator; }
}

// Grid division - determines snap resolution
enum GridDivision {
    WHOLE(1, false),           // 1/1
    HALF(2, false),            // 1/2
    QUARTER(4, false),         // 1/4
    EIGHTH(8, false),          // 1/8
    EIGHTH_TRIPLET(8, true),   // 1/8T
    SIXTEENTH(16, false),      // 1/16
    SIXTEENTH_TRIPLET(16, true), // 1/16T
    THIRTY_SECOND(32, false),  // 1/32
    THIRTY_SECOND_TRIPLET(32, true), // 1/32T
    SIXTY_FOURTH(64, false);   // 1/64

    private final int division;
    private final boolean triplet;

    GridDivision(int division, boolean triplet) {
        this.division = division;
        this.triplet = triplet;
    }

    // Ticks per grid unit at 96 PPQN
    public int ticksPerUnit() {
        int baseTicks = 96 * 4 / division;  // 96 PPQN, 4 quarters per whole
        return triplet ? baseTicks * 2 / 3 : baseTicks;
    }

    public String displayName() {
        return "1/" + division + (triplet ? "T" : "");
    }

    public GridDivision finer() {
        return switch (this) {
            case WHOLE -> HALF;
            case HALF -> QUARTER;
            case QUARTER -> EIGHTH;
            case EIGHTH -> SIXTEENTH;
            case SIXTEENTH -> THIRTY_SECOND;
            case THIRTY_SECOND -> SIXTY_FOURTH;
            case SIXTY_FOURTH -> SIXTY_FOURTH; // can't go finer
            case EIGHTH_TRIPLET -> SIXTEENTH_TRIPLET;
            case SIXTEENTH_TRIPLET -> THIRTY_SECOND_TRIPLET;
            case THIRTY_SECOND_TRIPLET -> THIRTY_SECOND_TRIPLET;
        };
    }

    public GridDivision coarser() {
        return switch (this) {
            case SIXTY_FOURTH -> THIRTY_SECOND;
            case THIRTY_SECOND -> SIXTEENTH;
            case SIXTEENTH -> EIGHTH;
            case EIGHTH -> QUARTER;
            case QUARTER -> HALF;
            case HALF -> WHOLE;
            case WHOLE -> WHOLE; // can't go coarser
            case THIRTY_SECOND_TRIPLET -> SIXTEENTH_TRIPLET;
            case SIXTEENTH_TRIPLET -> EIGHTH_TRIPLET;
            case EIGHTH_TRIPLET -> EIGHTH_TRIPLET;
        };
    }

    public GridDivision toggleTriplet() {
        return switch (this) {
            case EIGHTH -> EIGHTH_TRIPLET;
            case EIGHTH_TRIPLET -> EIGHTH;
            case SIXTEENTH -> SIXTEENTH_TRIPLET;
            case SIXTEENTH_TRIPLET -> SIXTEENTH;
            case THIRTY_SECOND -> THIRTY_SECOND_TRIPLET;
            case THIRTY_SECOND_TRIPLET -> THIRTY_SECOND;
            default -> this; // no triplet variant
        };
    }
}

// Zoom level - how many bars visible
enum ZoomLevel {
    BARS_1(1),
    BARS_2(2),
    BARS_4(4),
    BARS_8(8),
    BARS_16(16);

    private final int bars;

    ZoomLevel(int bars) { this.bars = bars; }

    public int bars() { return bars; }

    public String displayName() { return bars + (bars == 1 ? " bar" : " bars"); }

    public ZoomLevel zoomIn() {
        return switch (this) {
            case BARS_16 -> BARS_8;
            case BARS_8 -> BARS_4;
            case BARS_4 -> BARS_2;
            case BARS_2 -> BARS_1;
            case BARS_1 -> BARS_1; // can't zoom in more
        };
    }

    public ZoomLevel zoomOut() {
        return switch (this) {
            case BARS_1 -> BARS_2;
            case BARS_2 -> BARS_4;
            case BARS_4 -> BARS_8;
            case BARS_8 -> BARS_16;
            case BARS_16 -> BARS_16; // can't zoom out more
        };
    }
}

// Global musical settings
record MusicalSettings(
    MusicalKey key,
    TimeSignature meter,
    double tempoBpm,
    double tuningHz,
    GridDivision grid,
    ZoomLevel zoom,
    boolean snapToGrid,
    boolean snapToKey
) {
    public static final int PPQN = 96;  // Pulses per quarter note

    public static MusicalSettings defaults() {
        return new MusicalSettings(
            MusicalKey.Am(),
            TimeSignature.FOUR_FOUR,
            120.0,
            440.0,
            GridDivision.SIXTEENTH,  // 1/16 default grid
            ZoomLevel.BARS_2,        // 2 bars visible
            true,                     // snap to grid ON
            true                      // snap to key ON
        );
    }

    // Convert MIDI note to frequency using tuning reference
    public double midiToFreq(int midiNote) {
        return tuningHz * Math.pow(2.0, (midiNote - 69) / 12.0);
    }

    // Convert ticks to musical position
    public String ticksToPosition(int ticks) {
        int ticksPerBar = PPQN * 4 * meter.numerator() / meter.denominator();
        int bar = ticks / ticksPerBar + 1;
        int tickInBar = ticks % ticksPerBar;
        int beat = tickInBar / PPQN + 1;
        int tickInBeat = tickInBar % PPQN;
        return String.format("%d.%d.%02d", bar, beat, tickInBeat);
    }

    // Snap ticks to current grid
    public int snapToGrid(int ticks) {
        int gridTicks = grid.ticksPerUnit();
        return Math.round((float) ticks / gridTicks) * gridTicks;
    }
}

// Per-track key override (in Track record)
record Track(
    List<Step> steps,
    TrackTarget target,
    MusicalKey keyOverride  // null = use global
) {
    public MusicalKey effectiveKey(MusicalSettings global) {
        return keyOverride != null ? keyOverride : global.key();
    }
}
```

## Implementation Phases

### Phase 1: Core Types
- Note enum
- Scale enum with interval patterns
- MusicalKey record with contains(), snap(), nextInKey()
- TimeSignature record
- MusicalSettings record

### Phase 2: State Integration
- Add MusicalSettings to Sequencer or RackState
- Add keyOverride to Track
- StateTransitions for settings changes

### Phase 3: UI
- Render settings bar in SequencerViewRenderer
- Visual indicators for in-key/out-of-key/root
- Settings edit mode

### Phase 4: Behavior
- Snap-to-key on pitch navigation
- Quantize command
- Shift modifier for chromatic override

### Phase 5: Playback
- Tempo affects clock speed
- Tuning affects frequency calculation
- Meter affects metronome/visual

## Future Enhancements

- **Chord detection** - highlight chord tones differently
- **Scale suggestions** - based on existing notes
- **Modulation** - key changes mid-sequence
- **Microtuning** - custom tuning tables beyond 12-TET
- **Polyrhythm** - per-track meter overrides
