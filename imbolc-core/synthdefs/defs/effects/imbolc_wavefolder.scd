// imbolc_wavefolder SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_wavefolder, { |in=1024, out=1026, drive=1, symmetry=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var offset = (symmetry.lag(lag) - 0.5) * 0.5;
    var driven = (sig + offset) * drive.lag(lag);
    var folded = driven.fold2(1);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (folded * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
