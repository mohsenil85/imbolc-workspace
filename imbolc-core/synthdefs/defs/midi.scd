// MIDI - Note source that outputs freq/gate/vel on control buses
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

SynthDef(\imbolc_midi, { |freq_out=0, gate_out=0, vel_out=0, note=60, freq=440, vel=0.8, gate=1|
    var env = EnvGen.kr(Env.asr(0.001, 1, 0.01), gate, doneAction: 2);
    ReplaceOut.kr(freq_out, freq);
    ReplaceOut.kr(gate_out, env);
    ReplaceOut.kr(vel_out, vel);
}).writeDefFile(dir);
)
