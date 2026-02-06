// imbolc_flanger SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_flanger, { |in=1024, out=1026, rate=0.3, depth=0.003, feedback=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mod = SinOsc.kr(rate) * depth;
    var delayTime = (depth + mod).clip(0.0001, 0.01);
    var wet = CombL.ar(sig, 0.01, delayTime, feedback.lag(lag) * 0.5);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
