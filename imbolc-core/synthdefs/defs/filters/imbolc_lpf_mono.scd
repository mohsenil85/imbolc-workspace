// imbolc_lpf_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_lpf_mono, { |in=1024, out=1026, cutoff_mod_in=(-1), res_mod_in=(-1), cutoff=1000, resonance=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalCutoff = (cutoff.lag(lag) * (1 + cutoffMod)).clip(20, 20000);
    var finalRes = (resonance.lag(lag) + resMod).clip(0, 1);
    var q = finalRes.linlin(0, 1, 1, 0.1);
    Out.ar(out, RLPF.ar(sig, finalCutoff, q) * fadein);
}).writeDefFile(dir);
)
