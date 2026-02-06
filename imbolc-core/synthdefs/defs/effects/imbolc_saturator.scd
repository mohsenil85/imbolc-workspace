// imbolc_saturator SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_saturator, { |in=1024, out=1026, drive=1.5, color=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var saturated = (sig * drive.lag(lag)).tanh;
    var toneFreq = color.lag(lag).linexp(0, 1, 500, 15000);
    var filtered = LPF.ar(saturated, toneFreq);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (filtered * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
