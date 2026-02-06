// imbolc_chorus_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_chorus_mono, { |in=1024, out=1026, rate=0.5, depth=0.005, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var mod = SinOsc.kr(rate, 0) * depth;
    var wet = DelayL.ar(sig, 0.05, (depth + mod).clip(0.0001, 0.05));
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
