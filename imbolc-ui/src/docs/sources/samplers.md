# Samplers

Sample playback and manipulation.

## Pitched Sampler

Plays a sample pitched across the keyboard.

**Parameters:**
- Sample: Source audio file
- Root Key: Original pitch of sample
- Loop: Enable/disable looping
- Loop Start/End: Loop points

**Tips:**
- Set root key to match the sample's original pitch
- Use loop for sustaining sounds (pads, strings)
- One-shot mode for drums and hits

## Time Stretch

Granular time-stretching sampler. Change tempo without affecting pitch.

**Parameters:**
- Sample: Source audio file
- Stretch: Time stretch factor
- Grain Size: Granular window size
- Pitch: Independent pitch control

**Tips:**
- Larger grains = smoother but less responsive
- Smaller grains = more accurate but potential artifacts
- Extreme stretch values create textural effects

## Kit

Drum kit sampler with multiple pads.

**Parameters:**
- Pads: 12 sample slots
- Per-pad: Level, Pan, Pitch, Reverse

**Workflow:**
1. Load samples to pads using the Sample Chopper
2. Sequence with the Drum Sequencer (F2)
3. Adjust per-pad settings as needed

**Tips:**
- Assign related sounds to adjacent pads
- Use pitch for variation without loading multiple samples
- Reverse for interesting textures

## Granular

Granular synthesis sampler.

**Parameters:**
- Sample: Source audio file
- Position: Playback position in sample
- Grain Size: Duration of each grain
- Density: Grains per second
- Spray: Random position variation
- Pitch Spray: Random pitch variation

**Tips:**
- Small grains + high density = smooth texture
- Large grains = audible chunks
- Position modulation creates evolving textures
- High spray values = glitchy, scattered sound
