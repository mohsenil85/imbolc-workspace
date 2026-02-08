// imbolc_tuner_tone SynthDef
// Simple sine wave reference tone for instrument tuning.
// Outputs directly to hardware out (bus 0), bypassing instrument routing.
// gate=1 to play, gate=0 to release (50ms fade, self-frees via doneAction:2).
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_tuner_tone, { |out=0, freq=440, amp=0.3, gate=1|
    var sig, env;
    env = EnvGen.kr(Env.asr(0.01, 1, 0.05), gate, doneAction: 2);
    sig = SinOsc.ar(freq, 0, amp);
    Out.ar(out, sig ! 2);
}).writeDefFile(dir);
)
