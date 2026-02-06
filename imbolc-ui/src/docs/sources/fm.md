# FM Synthesis

Frequency modulation and related modulation techniques.

## FM

Classic two-operator FM synthesis. One oscillator (modulator) modulates
the frequency of another (carrier), creating complex harmonic spectra.

**Parameters:**
- Ratio: Carrier-to-modulator frequency ratio
- Index: Modulation depth (higher = more harmonics)
- Feedback: Modulator self-modulation

**Tips:**
- Integer ratios produce harmonic tones
- Non-integer ratios create inharmonic, bell-like tones
- Higher index values add brightness and complexity

## Ring Mod

Ring modulation multiplies two signals, creating sum and difference
frequencies. Metallic, robotic tones.

**Parameters:**
- Freq: Modulator frequency
- Mix: Dry/wet blend

**Tips:**
- Low modulator frequencies create tremolo
- High frequencies create metallic timbres
- Non-harmonic modulators create clangorous tones

## Feedback Sine

Sine oscillator with adjustable self-feedback. From pure sine to
harsh distortion as feedback increases.

**Parameters:**
- Feedback: Self-modulation amount
- Character: Feedback waveshaping

**Tips:**
- Low feedback: warm sine with slight harmonics
- Medium feedback: increasingly bright and aggressive
- High feedback: noisy, chaotic tones

## Phase Mod

Phase modulation synthesis, similar to FM but with different
mathematical properties. Used in classic digital synths.

**Parameters:**
- Ratio: Modulator ratio
- Depth: Modulation amount

## FM Bell

Pre-configured FM patch optimized for bell-like tones.

**Parameters:**
- Decay: Bell ring time
- Brightness: High harmonic content
- Inharmonicity: Detuning for realistic bell character

**Tips:**
- Works well with long release times
- Try velocity-to-index modulation for expression

## FM Brass

FM patch designed for brass-like sounds with complex attack.

**Parameters:**
- Attack: Initial brightness burst
- Bite: Transient sharpness
- Body: Sustain character

**Tips:**
- Use envelope on FM index for realistic brass attacks
- Layer with filter envelope for extra punch
