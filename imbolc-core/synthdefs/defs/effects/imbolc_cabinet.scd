// imbolc_cabinet SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_cabinet, { |in=1024, out=1026, type=0, tone=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var toneFreq = tone.lag(lag);
    // Different cabinet profiles with different LPF cutoffs and resonance
    var small = LPF.ar(sig.tanh, toneFreq.linexp(0, 1, 800, 5000));
    var large = LPF.ar(sig.tanh, toneFreq.linexp(0, 1, 600, 4000));
    var stack = LPF.ar(BPeakEQ.ar(sig.tanh, 2000, 1, 3), toneFreq.linexp(0, 1, 500, 3500));
    var open = LPF.ar(sig.tanh, toneFreq.linexp(0, 1, 1000, 6000));
    var wet = Select.ar(type, [small, large, stack, open]);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
