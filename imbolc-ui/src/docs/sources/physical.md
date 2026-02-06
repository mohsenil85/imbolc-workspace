# Physical Models

Physically modeled instrument simulations.

## Pluck

Karplus-Strong string synthesis. A burst of noise filtered through
a tuned delay creates realistic plucked string sounds.

**Parameters:**
- Decay: String damping (how quickly it fades)
- Brightness: Initial excitation brightness
- Position: Pick position along string

**Tips:**
- Short decay = muted guitar, long decay = sustaining strings
- Lower brightness = softer pluck, higher = sharper attack

## Formant

Vocal formant synthesis for voice-like tones.

**Parameters:**
- Vowel: Formant shape (A, E, I, O, U)
- Formant Shift: Pitch of formants independent of note
- Breathiness: Noise content

## Bowed

Bowed string physical model (violin, cello, etc).

**Parameters:**
- Bow Pressure: Force against string
- Bow Position: Distance from bridge
- Vibrato: Pitch modulation

**Tips:**
- Higher pressure = louder, more aggressive
- Position near bridge = brighter, "sul ponticello"

## Blown

Wind instrument model (flute, clarinet, etc).

**Parameters:**
- Breath: Air pressure
- Embouchure: Mouth position/tension
- Noise: Breath noise

## Membrane

Drum head / membrane physical model.

**Parameters:**
- Tension: Membrane tightness (pitch)
- Damping: How quickly vibration decays
- Strike Position: Where the membrane is hit

## Marimba

Wooden bar percussion with resonator.

**Parameters:**
- Hardness: Mallet hardness (soft felt to hard plastic)
- Resonance: Resonator tube length

## Vibes

Vibraphone with motor-driven tremolo.

**Parameters:**
- Motor Speed: Tremolo rate
- Damper: Sustain pedal amount

## Kalimba

Thumb piano / mbira model.

**Parameters:**
- Tine Length: Affects timbre
- Body Resonance: Wooden body contribution

## Steel Drum

Caribbean steel pan model.

**Parameters:**
- Damping: Note decay
- Brightness: Harmonic content

## Tubular Bell

Orchestral chime/tubular bell.

**Parameters:**
- Size: Bell diameter
- Strike: Hit position

## Glockenspiel

Metal bar percussion with bright, bell-like tone.

**Parameters:**
- Hardness: Mallet hardness
- Damping: Ring time

## Guitar

Nylon or steel string acoustic guitar model.

**Parameters:**
- Body: Resonance of guitar body
- Pick Position: Neck to bridge
- String Type: Nylon/steel character

## Bass Guitar

Electric bass guitar model.

**Parameters:**
- Pickup: Position simulation
- Tone: Brightness control

## Harp

Concert harp model with multiple strings.

**Parameters:**
- Damping: String ring time
- Resonance: Soundboard character

## Koto

Japanese plucked string instrument.

**Parameters:**
- Bridge Position: Moveable bridge placement
- Pluck Style: Nail vs flesh
