// imbolc_resonator SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_resonator, { |in=1024, out=1026, freq=440, decay=1.0, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var delay = freq.lag(lag).reciprocal.clip(0.00005, 0.05);
    var wet = CombC.ar(sig, 0.05, delay, decay.lag(lag));
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
