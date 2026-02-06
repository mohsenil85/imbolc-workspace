// imbolc_allpass SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_allpass, { |in=1024, out=1026, cutoff_mod_in=(-1), res_mod_in=(-1), cutoff=440, resonance=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalCutoff = (cutoff.lag(lag) * (1 + cutoffMod)).clip(20, 20000);
    var finalRes = (resonance.lag(lag) + resMod).clip(0, 1);
    var delay = finalCutoff.reciprocal.clip(0.00005, 0.05);
    var decayTime = finalRes.linlin(0, 1, 0.01, 2.0);
    Out.ar(out, AllpassC.ar(sig, 0.05, delay, decayTime) * fadein);
}).writeDefFile(dir);
)
