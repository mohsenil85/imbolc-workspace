# Routing

Audio routing and external input sources.

## Audio In

Routes external audio input through the instrument's processing chain.

**Parameters:**
- Input: Select audio input channel(s)
- Gain: Input level adjustment
- Monitor: Enable/disable direct monitoring

**Use Cases:**
- Process external synths through Imbolc effects
- Guitar/bass processing
- Vocal processing
- Live sampling source

**Tips:**
- Set appropriate input gain to avoid clipping
- Use the instrument's filter and effects for processing
- Combine with automation for dynamic processing

## Bus In

Receives audio from an internal bus for reprocessing.

**Parameters:**
- Bus: Source bus selection
- Gain: Input level

**Use Cases:**
- Parallel processing (send to bus, process differently)
- Sidechain source for effects
- Complex routing for layered sounds
- Re-amping internal audio

**Workflow:**
1. Set up a send from source instrument to a bus
2. Create BusIn instrument receiving from that bus
3. Add different effects/processing
4. Mix with original or use alone

**Tips:**
- Great for parallel compression
- Use for creative effects routing
- Enables complex sound design with multiple processing paths
