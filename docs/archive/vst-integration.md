# VST Integration via SuperCollider VSTPlugin

Legacy note: this document captures an older prototype and is superseded by
`docs/vst3-support-roadmap.md` for the current Rust codebase and roadmap.

Load VST/VST3 plugins through SuperCollider's VSTPlugin extension.

## Overview

VST plugins are native code (C++), so Java can't load them directly. Instead:

1. SuperCollider loads plugins via VSTPlugin extension
2. TUI DAW sends OSC to control VST parameters
3. Audio flows through SC's bus system like other modules

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  TUI DAW (Java)                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  VST Module State                                        │   │
│  │  - pluginPath: "/Library/Audio/Plug-Ins/VST3/Serum.vst3"│   │
│  │  - params: Map<String, Double>                          │   │
│  │  - presetPath: "Bass Wobble.fxp"                        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                           │ OSC                                 │
│                           ▼                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  SuperCollider Server                                    │   │
│  │  ┌───────────────────────────────────────────────────┐  │   │
│  │  │  VSTPlugin Synth                                   │  │   │
│  │  │  ┌─────────────────────────────────────────────┐  │  │   │
│  │  │  │  Serum.vst3                                  │  │  │   │
│  │  │  │  - Audio In ← from TUI DAW modules          │  │  │   │
│  │  │  │  - Audio Out → to mixer channel             │  │  │   │
│  │  │  │  - MIDI In ← from sequencer                 │  │  │   │
│  │  │  └─────────────────────────────────────────────┘  │  │   │
│  │  └───────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Prerequisites

### Install VSTPlugin Extension

```bash
# In SuperCollider IDE
Quarks.install("VSTPlugin");
VSTPlugin.search;  # Scan for installed plugins
```

### Verify Installation

```supercollider
// List available plugins
VSTPlugin.pluginList.do { |p| p.postln };

// Test loading a plugin
(
SynthDef(\vstTest, {
    var sig = VSTPlugin.ar(nil, 2);
    Out.ar(0, sig);
}).add;
)

x = Synth(\vstTest);
c = VSTPluginController(x);
c.open("Serum", { |ctrl| "Plugin loaded!".postln });
```

## SuperCollider SynthDef

```supercollider
// SynthDef for VST instrument (synth)
SynthDef(\vstInstrument, {
    |out=0, midiChan=0|
    var sig = VSTPlugin.ar(nil, 2, id: \vst);
    Out.ar(out, sig);
}).add;

// SynthDef for VST effect (processor)
SynthDef(\vstEffect, {
    |out=0, in=0|
    var input = In.ar(in, 2);
    var sig = VSTPlugin.ar(input, 2, id: \vst);
    Out.ar(out, sig);
}).add;
```

## OSC Protocol

### Load Plugin

```
/s_new "vstInstrument" <nodeId> 0 0 "out" <bus>
/vst_open <nodeId> "Serum.vst3"
```

### Set Parameter

```
/vst_set <nodeId> <paramIndex> <value>
/vst_setn <nodeId> <paramIndex> <count> <value1> <value2> ...
```

### Load Preset

```
/vst_program_read <nodeId> "/path/to/preset.fxp"
```

### Send MIDI

```
/vst_midi <nodeId> <status> <data1> <data2>
// e.g., Note On: /vst_midi 12345 144 60 100
```

### Get Parameter Info

```
/vst_query <nodeId>  # Returns parameter names and ranges
```

## Java Implementation

### ModuleType

```java
public enum ModuleType {
    // ... existing types ...
    VST_INSTRUMENT("vsti"),  // Synth/sampler VST
    VST_EFFECT("vstfx");     // Effect VST
}
```

### VSTModule State

```java
public record VSTModule(
    String id,
    String pluginPath,          // Full path to .vst3 or .vst
    String pluginName,          // Display name
    Map<String, Double> params, // Current param values
    String presetPath,          // Loaded preset (nullable)
    List<VSTParam> paramInfo    // Metadata from plugin
) {}

public record VSTParam(
    int index,
    String name,
    double minValue,
    double maxValue,
    double defaultValue,
    String unit
) {}
```

### OSCClient Extensions

```java
public class OSCClient {
    // ... existing methods ...

    public void vstOpen(int nodeId, String pluginPath) {
        send("/vst_open", nodeId, pluginPath);
    }

    public void vstSet(int nodeId, int paramIndex, float value) {
        send("/vst_set", nodeId, paramIndex, value);
    }

    public void vstMidi(int nodeId, int status, int data1, int data2) {
        send("/vst_midi", nodeId, status, data1, data2);
    }

    public void vstLoadPreset(int nodeId, String presetPath) {
        send("/vst_program_read", nodeId, presetPath);
    }

    public void vstSavePreset(int nodeId, String presetPath) {
        send("/vst_program_write", nodeId, presetPath);
    }
}
```

### Plugin Scanner

```java
public class VSTScanner {
    private static final List<String> SEARCH_PATHS = List.of(
        "/Library/Audio/Plug-Ins/VST3",
        "/Library/Audio/Plug-Ins/VST",
        "~/Library/Audio/Plug-Ins/VST3",
        "~/Library/Audio/Plug-Ins/VST"
    );

    public List<VSTPluginInfo> scan() {
        // Send /vst_search to SC, parse response
        // Or read from cached VSTPlugin.pluginList
    }
}
```

## UI Integration

### Plugin Browser

New view: `View.VST_BROWSER`

```
┌─ VST Browser ──────────────────────────────────────────────────┐
│                                                                │
│  Instruments:                                                  │
│  > Serum                    /Library/.../VST3/Serum.vst3      │
│    Vital                    /Library/.../VST3/Vital.vst3      │
│    Diva                     /Library/.../VST3/Diva.vst3       │
│                                                                │
│  Effects:                                                      │
│    FabFilter Pro-Q 3        /Library/.../VST3/Pro-Q 3.vst3    │
│    Valhalla Room            /Library/.../VST3/ValhallaRoom... │
│                                                                │
│  [Enter] Load  [/] Search  [Esc] Cancel                       │
└────────────────────────────────────────────────────────────────┘
```

### VST Module in Rack View

```
┌─ vsti-1: Serum ────────────────────┐
│  Preset: Bass Wobble               │
│  Cutoff:     ████████░░ 0.75      │
│  Resonance:  ██████░░░░ 0.60      │
│  [more params...]                  │
│                                    │
│  [e] Edit in Plugin UI             │
└────────────────────────────────────┘
```

### Plugin UI (Future)

VSTPlugin can open the native plugin GUI:

```supercollider
c.editor;  // Opens plugin's native window
```

This requires X11 forwarding or native window integration, which is complex in a TUI app. Initial implementation: parameter editing only in TUI.

## Sequencer Integration

VST instruments receive MIDI from sequencer:

```java
// In SequencerClock, when step triggers:
if (targetModule.type() == ModuleType.VST_INSTRUMENT) {
    int nodeId = rack.getNodeId(targetModule.id());
    int note = step.pitch();
    int velocity = (int)(step.velocity() * 127);

    // Note On
    oscClient.vstMidi(nodeId, 0x90, note, velocity);

    // Schedule Note Off after gate time
    scheduler.schedule(() -> {
        oscClient.vstMidi(nodeId, 0x80, note, 0);
    }, gateTimeMs, TimeUnit.MILLISECONDS);
}
```

## Limitations

1. **No native GUI in TUI**: Must edit params via TUI controls
2. **Plugin scanning**: Requires SC running to scan
3. **Parameter discovery**: Need to query SC for param names/ranges
4. **Latency**: Additional hop through SC adds latency
5. **Stability**: Buggy plugins can crash SC server

## Alternatives Considered

| Approach | Pros | Cons |
|----------|------|------|
| **VSTPlugin (chosen)** | Mature, SC integration | No native GUI |
| Carla bridge | Any plugin, GUI | Extra process |
| JNI + JUCE | Direct, fast | Huge undertaking |
| LV2 only | Open standard | Linux only |

## Implementation Phases

1. **Phase 1**: Basic VST loading and audio routing
2. **Phase 2**: Parameter editing in TUI
3. **Phase 3**: Preset management
4. **Phase 4**: MIDI from sequencer
5. **Phase 5**: Plugin browser/scanner
6. **Future**: Native GUI via IPC
