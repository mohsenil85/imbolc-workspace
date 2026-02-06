// Fat Bass - A thick, detuned bass synth for imbolc
// Custom params: detune, sub_level, cutoff, resonance

SynthDef(\fat_bass, {
    |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    attack=0.01, decay=0.1, sustain=0.8, release=0.3,
    detune=0.15, sub_level=0.5, cutoff=800, resonance=0.3|

    // Standard input handling
    var freqSig = Select.kr(freq_in >= 0, [110, In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);

    // Three detuned saw oscillators for fatness
    var osc1 = Saw.ar(freqSig);
    var osc2 = Saw.ar(freqSig * (1 + (detune * 0.01)));
    var osc3 = Saw.ar(freqSig * (1 - (detune * 0.01)));

    // Sub oscillator (one octave down)
    var sub = SinOsc.ar(freqSig * 0.5) * sub_level;

    // Mix oscillators
    var mix = (osc1 + osc2 + osc3) / 3 + sub;

    // Low pass filter with envelope
    var filterEnv = EnvGen.kr(Env.adsr(attack * 0.5, decay, sustain * 0.7, release * 0.5), gateSig);
    var filtered = RLPF.ar(mix, cutoff * filterEnv.linexp(0, 1, 0.5, 2), resonance.linlin(0, 1, 1, 0.1));

    // Amplitude envelope
    var env = EnvGen.kr(Env.adsr(attack, decay, sustain, release), gateSig);

    // Output
    var sig = filtered * env * velSig * 0.4;
    Out.ar(out, sig ! 2);
}).writeDefFile(thisProcess.nowExecutingPath.dirname);
