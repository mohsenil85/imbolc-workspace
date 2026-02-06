// imbolc_phaser SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_phaser, { |in=1024, out=1026, rate=0.5, depth=0.5, stages=4, feedback=0.3, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var lfo = SinOsc.kr(rate).range(200, 4000);
    var wet = sig;
    // 6 allpass stages, gated by stages param
    6.do { |i|
        var active = (i < stages);
        var freq = lfo * ((i + 1) / 6);
        var delay = freq.reciprocal.clip(0.00001, 0.01);
        wet = Select.ar(active, [wet, AllpassL.ar(wet, 0.01, delay, 0.001)]);
    };
    wet = wet + (LocalIn.ar(2) * feedback.lag(lag));
    LocalOut.ar(wet);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
