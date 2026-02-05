# AI Integration with Haiku

## Overview

Use Claude Haiku as a lightweight, fast AI backend for natural language sound design commands. Haiku parses user intent and returns structured actions that the app executes.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ User types: "make me a piano pad"                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ App builds context:                                             │
│ {                                                               │
│   "command": "make me a piano pad",                             │
│   "rack_state": { modules: [...], patches: [...] },             │
│   "available_modules": ["SAW_SOURCE", "LPF", "REVERB", ...],    │
│   "sequencer": { bpm: 120, steps: 16, ... }                     │
│ }                                                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Haiku API Call (fast, cheap)                                    │
│ System prompt defines available actions and sound design rules  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Haiku returns structured actions:                               │
│ {                                                               │
│   "actions": [                                                  │
│     {"type": "add_module", "module": "SAW_SOURCE",              │
│      "params": {"freq": 220, "amp": 0.3}},                      │
│     {"type": "add_module", "module": "SAW_SOURCE",              │
│      "params": {"freq": 221, "amp": 0.3}},                      │
│     {"type": "add_module", "module": "LPF",                     │
│      "params": {"cutoff": 800}},                                │
│     {"type": "add_module", "module": "REVERB",                  │
│      "params": {"room": 0.8, "mix": 0.4}},                      │
│     {"type": "connect", "from": "saw-1:out", "to": "lpf-1:in"}, │
│     {"type": "connect", "from": "saw-2:out", "to": "lpf-1:in"}, │
│     {"type": "connect", "from": "lpf-1:out", "to": "verb-1:in"} │
│   ],                                                            │
│   "explanation": "Created a pad with detuned saws, filtered..." │
│ }                                                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ App executes actions sequentially                               │
│ Shows explanation to user                                       │
└─────────────────────────────────────────────────────────────────┘
```

## Command Types

### Module Creation
```
"make me a sine wave source"
"add a low pass filter"
"I need a reverb"
```
→ Single module with sensible defaults

### Patch/Preset Creation
```
"make me a piano pad"
"create a thick bass sound"
"build a plucky lead"
```
→ Multiple modules configured and connected:
- Piano pad: detuned sources + LPF + slow envelope + reverb
- Thick bass: saw source + LPF with resonance + sub source
- Plucky lead: saw + fast envelope + slight delay

### Sequencer Patterns
```
"make a four on the floor beat"
"create a breakbeat pattern"
"give me a hi-hat pattern with swing"
```
→ Sequencer pattern data:
```json
{
  "actions": [
    {"type": "set_pattern", "track": 0, "steps": [1,0,0,0, 1,0,0,0, 1,0,0,0, 1,0,0,0]},
    {"type": "set_pattern", "track": 1, "steps": [0,0,1,0, 0,0,1,0, 0,0,1,0, 0,0,1,0]},
    {"type": "set_swing", "amount": 0.6, "grid": "16ths"}
  ]
}
```

### Sound Adjustment
```
"make it warmer"
"less bright"
"more spacious"
"add some grit"
```
→ Parameter adjustments on existing modules

### Patching
```
"connect the saw to the filter"
"route the LFO to the filter cutoff"
"send it through the reverb"
```
→ Infers module IDs, creates connections

## Action Schema

```json
{
  "actions": [
    {
      "type": "add_module",
      "module": "SAW_SOURCE | LPF | LFO | REVERB | ...",
      "id": "optional-custom-id",
      "params": { "freq": 440, "amp": 0.5 }
    },
    {
      "type": "remove_module",
      "id": "saw-1"
    },
    {
      "type": "set_param",
      "module": "saw-1",
      "param": "freq",
      "value": 880
    },
    {
      "type": "adjust_param",
      "module": "saw-1",
      "param": "freq",
      "direction": "increase | decrease",
      "amount": "subtle | moderate | large"
    },
    {
      "type": "connect",
      "from": "saw-1:out",
      "to": "lpf-1:in"
    },
    {
      "type": "disconnect",
      "from": "saw-1:out",
      "to": "lpf-1:in"
    },
    {
      "type": "set_pattern",
      "track": 0,
      "steps": [1, 0, 0, 0, 1, 0, 0, 0, ...]
    },
    {
      "type": "set_tempo",
      "bpm": 120
    },
    {
      "type": "set_swing",
      "amount": 0.66,
      "grid": "8ths | 16ths | both"
    }
  ],
  "explanation": "Human-readable explanation of what was done"
}
```

## Haiku System Prompt

```
You are an AI assistant for a modular synthesizer. You receive natural language
commands and return structured actions as JSON.

Available modules:
- SAW_SOURCE: Sawtooth source (params: freq 20-20000, amp 0-1)
- LPF: Low-pass filter (params: cutoff 20-20000, res 0-1)
- LFO: Low frequency oscillator (params: rate 0.01-100, depth 0-1)
- REVERB: Reverb effect (params: room 0-1, damp 0-1, mix 0-1)
- DELAY_FX: Delay effect (params: time 0.01-2, feedback 0-0.95, mix 0-1)
- ENVELOPE: ADSR envelope (params: attack, decay, sustain, release)
- NOISE: Noise generator (params: amp 0-1, color 0-1 where 0=white, 1=pink)
- MIXER: 4-channel mixer with pan
- OUTPUT: Final output to speakers

Sound design guidelines:
- "warm": lower cutoff, slight resonance, maybe saturation
- "bright": higher cutoff, presence boost
- "spacious": reverb with high room, delay with moderate feedback
- "punchy": fast attack, short decay
- "pad": slow attack, long release, detuned sources, reverb
- "bass": low frequency, filtered, maybe sub source
- "lead": mid-high frequency, some brightness, slight delay

Always return valid JSON with "actions" array and "explanation" string.
```

## Implementation

### Java Classes

```
com.imbolc.ai/
├── AIClient.java        # HTTP client for Haiku API
├── AIContext.java       # Builds context from rack state
├── ActionParser.java    # Parses JSON response to Action objects
├── ActionExecutor.java  # Executes actions on Dispatcher/Rack
└── AIConfig.java        # API key, model settings
```

### AIClient.java (sketch)

```java
public class AIClient {
    private static final String API_URL = "https://api.anthropic.com/v1/messages";
    private final String apiKey;

    public CompletableFuture<AIResponse> sendCommand(String command, AIContext context) {
        // Build request with system prompt + user message
        // POST to Haiku API
        // Parse response
    }
}
```

### Integration with TUI

1. User presses `:` to enter AI mode
2. Types command, presses Enter
3. App shows "Thinking..." indicator
4. Sends to Haiku, receives response
5. Executes actions with visual feedback
6. Shows explanation

### Error Handling

- API timeout: "AI service unavailable, try again"
- Invalid response: "Couldn't understand AI response"
- Action failure: "Failed to create module: [reason]"
- Rate limiting: Queue requests, show "Please wait..."

## Configuration

```properties
# imbolc.properties
ai.enabled=true
ai.api_key=sk-ant-...
ai.model=claude-3-haiku-20240307
ai.timeout_ms=10000
```

Or environment variable:
```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

## Future Enhancements

### Conversation Memory
Remember context across commands:
- "make a bass sound" → creates bass
- "make it darker" → knows to adjust the bass
- "now add some movement" → adds LFO to bass filter

### Preset Library
- "save this as 'fat bass'"
- "load the preset called 'ambient pad'"
- Haiku can suggest presets: "Try the 'warm keys' preset"

### Learning User Preferences
- Track which adjustments user likes
- "warmer" might mean different things to different users
- Personalize responses over time

### Voice Input (Future)
- Whisper API for speech-to-text
- "Hey synth, make it groovier"
