// Modulation - ADSR Envelope and LFO
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

// ADSR Envelope - Gate input, control output
SynthDef(\imbolc_adsr, { |gate_in=(-1), out=0, attack=0.01, decay=0.1, sustain=0.7, release=0.3, gate=0|
    var gateSig = Select.kr(gate_in >= 0, [gate, In.kr(gate_in)]);
    var env = EnvGen.kr(Env.adsr(attack, decay, sustain, release), gateSig);
    Out.kr(out, env);
}).writeDefFile(dir);

// LFO - Control rate oscillator with multiple shapes
// shape: 0=sine, 1=square, 2=saw, 3=triangle
SynthDef(\imbolc_lfo, { |out=0, rate=1, depth=0.5, shape=0|
    var sine = SinOsc.kr(rate);
    var square = LFPulse.kr(rate).range(-1, 1);
    var saw = LFSaw.kr(rate);
    var tri = LFTri.kr(rate);
    var sig = Select.kr(shape, [sine, square, saw, tri]) * depth;
    Out.kr(out, sig);
}).writeDefFile(dir);
)
