// imbolc_tape_comp_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_tape_comp_mono, { |in=1024, out=1026, drive=1.5, threshold=0.5, ratio=3, makeup=1.0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var driven = (sig * drive.lag(lag)).tanh;
    var compressed = Compander.ar(driven, driven,
        thresh: threshold,
        slopeBelow: 1.0,
        slopeAbove: ratio.reciprocal,
        clampTime: 0.01,
        relaxTime: 0.1
    );
    var output = compressed * makeup.lag(lag);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (output * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
