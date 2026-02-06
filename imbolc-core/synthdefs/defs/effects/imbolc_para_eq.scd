// imbolc_para_eq SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_para_eq, { |in=1024, out=1026,
    lo_freq=200, lo_gain=0, mid_freq=1000, mid_gain=0, hi_freq=5000, hi_gain=0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    sig = BPeakEQ.ar(sig, lo_freq.lag(lag).clip(20, 20000), 1, lo_gain.lag(lag));
    sig = BPeakEQ.ar(sig, mid_freq.lag(lag).clip(20, 20000), 1, mid_gain.lag(lag));
    sig = BPeakEQ.ar(sig, hi_freq.lag(lag).clip(20, 20000), 1, hi_gain.lag(lag));
    Out.ar(out, sig * fadein);
}).writeDefFile(dir);
)
