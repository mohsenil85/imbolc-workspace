// imbolc_distortion SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_distortion, { |in=1024, out=1026, drive=2, mode=0, tone=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var driven = sig * drive.lag(lag);
    var hard = driven.clip2(1);
    var soft = driven.tanh;
    var fold = driven.fold2(1);
    var distorted = Select.ar(mode, [hard, soft, fold]);
    var toneFreq = tone.lag(lag).linexp(0, 1, 300, 12000);
    var filtered = LPF.ar(distorted, toneFreq);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (filtered * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
