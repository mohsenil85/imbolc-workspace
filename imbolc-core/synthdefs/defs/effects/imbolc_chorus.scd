// imbolc_chorus SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_chorus, { |in=1024, out=1026, rate=0.5, depth=0.005, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var modL = SinOsc.kr(rate, 0) * depth;
    var modR = SinOsc.kr(rate, 0.5pi) * depth;
    var wet = [
        DelayL.ar(sig[0], 0.05, (depth + modL).clip(0.0001, 0.05)),
        DelayL.ar(sig[1], 0.05, (depth + modR).clip(0.0001, 0.05))
    ];
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
