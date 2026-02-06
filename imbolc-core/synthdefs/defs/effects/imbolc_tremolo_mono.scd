// imbolc_tremolo_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_tremolo_mono, { |in=1024, out=1026, rate=4, depth=0.5, shape=0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var sine = SinOsc.kr(rate).range(1 - depth, 1);
    var square = LFPulse.kr(rate, width: 0.5).range(1 - depth, 1);
    var saw = LFSaw.kr(rate).range(1 - depth, 1);
    var lfo = Select.kr(shape, [sine, square, saw]);
    var wet = sig * lfo;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
