// imbolc_pitch_shifter SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_pitch_shifter, { |in=1024, out=1026, shift=0, window=0.2, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var ratio = 2 ** (shift.lag(lag) / 12); // semitones to ratio
    var wet = PitchShift.ar(sig, window.lag(lag), ratio);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
