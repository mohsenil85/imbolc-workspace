// imbolc_ringmod SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_ringmod, { |in=1024, out=1026, freq=440, depth=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mod = SinOsc.ar(freq.lag(lag));
    var wet = sig * ((1 - depth.lag(lag)) + (mod * depth.lag(lag)));
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
