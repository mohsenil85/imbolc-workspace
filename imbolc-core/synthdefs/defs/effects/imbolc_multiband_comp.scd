// imbolc_multiband_comp SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_multiband_comp, { |in=1024, out=1026, low_thresh=0.3, mid_thresh=0.3, hi_thresh=0.3, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var lo = LPF.ar(sig, 250);
    var mid = BPF.ar(sig, 2000, 1);
    var hi = HPF.ar(sig, 6000);
    var wet = Compander.ar(lo, lo, low_thresh.lag(lag), 1, 0.25, 0.01, 0.1)
            + Compander.ar(mid, mid, mid_thresh.lag(lag), 1, 0.25, 0.01, 0.1)
            + Compander.ar(hi, hi, hi_thresh.lag(lag), 1, 0.25, 0.01, 0.1);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
