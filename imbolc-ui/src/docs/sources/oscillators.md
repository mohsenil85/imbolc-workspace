# Oscillators

Basic waveform generators for sound synthesis.

## Saw

A sawtooth wave with all harmonics. Bright, buzzy tone that's great for
leads, basses, and pads. The classic subtractive synth starting point.

**Parameters:**
- Detune: Slight pitch variation for thickness
- Phase: Starting phase of the waveform

## Sine

A pure sine wave with no harmonics. The simplest waveform, useful for
sub-bass, FM carriers, and pure tones.

**Parameters:**
- Phase: Starting phase

## Square

Equal on/off duty cycle producing odd harmonics only. Hollow, clarinet-like
tone. Good for leads and arpeggios.

**Parameters:**
- Phase: Starting phase

## Triangle

Softer than square, still odd harmonics but with faster rolloff. Flute-like
quality, good for soft leads and sub-bass.

**Parameters:**
- Phase: Starting phase

## Noise

White noise generator. Useful for hi-hats, snares, risers, and adding
texture to sounds.

**Parameters:**
- Color: Filter applied to noise

## Pulse

Variable pulse width for timbral control. At 50% it's a square wave.
Narrower pulses create thinner, more nasal tones.

**Parameters:**
- Width: Pulse width (0-100%)
- PWM: Pulse width modulation depth

## SuperSaw

Multiple detuned sawtooth oscillators for a huge, rich sound. Classic
trance and EDM lead sound.

**Parameters:**
- Voices: Number of oscillators (1-7)
- Detune: Spread between voices
- Mix: Blend of voices

## Sync

Hard-synced oscillator pair. The slave oscillator resets at the master's
frequency, creating harmonically rich, aggressive tones.

**Parameters:**
- Ratio: Frequency ratio of slave to master
- Mix: Blend of master and slave

## Choir

Vocal-like pad sound using formant synthesis.

**Parameters:**
- Vowel: Formant shape (a, e, i, o, u)
- Vibrato: Pitch modulation depth

## EPiano

Electric piano emulation with bell-like attack.

**Parameters:**
- Tone: Brightness
- Velocity: Dynamic response

## Organ

Drawbar organ simulation.

**Parameters:**
- Drawbars: Harmonic mix
- Percussion: Attack transient

## Brass Stab

Punchy brass section sound.

**Parameters:**
- Attack: Initial transient brightness
- Fatness: Low-frequency emphasis

## Strings

Ensemble string pad.

**Parameters:**
- Ensemble: Chorus/detuning amount
- Attack: String attack time

## Acid

Classic TB-303 style acid bass.

**Parameters:**
- Cutoff: Filter frequency
- Resonance: Filter emphasis
- Accent: Dynamic filter sweep

## Additive

Harmonic additive synthesis with individual partial control.

**Parameters:**
- Harmonics: Level of each partial
- Spread: Harmonic spacing

## Wavetable

Morphable wavetable synthesis.

**Parameters:**
- Position: Wavetable position
- Morph: Interpolation between frames

## Gendy

Stochastic waveform synthesis (Iannis Xenakis).

**Parameters:**
- Amp: Amplitude deviation
- Dur: Duration deviation

## Chaos

Chaotic oscillator based on nonlinear dynamics.

**Parameters:**
- Parameter: Chaos control
- Rate: Update frequency
