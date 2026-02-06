// imbolc_autopan SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_autopan, { |in=1024, out=1026, rate=2, depth=0.5, shape=0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var lfo = Select.kr(shape, [SinOsc.kr(rate), LFPulse.kr(rate).range(-1,1), LFTri.kr(rate)]) * depth.lag(lag);
    var wet = Pan2.ar(mono, lfo);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
