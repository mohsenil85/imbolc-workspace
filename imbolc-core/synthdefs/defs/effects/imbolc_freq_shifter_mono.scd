// imbolc_freq_shifter_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_freq_shifter_mono, { |in=1024, out=1026, shift_hz=0, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var wet = FreqShift.ar(sig, shift_hz.lag(lag));
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
