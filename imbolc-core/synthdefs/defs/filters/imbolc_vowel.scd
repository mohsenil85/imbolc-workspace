// imbolc_vowel SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_vowel, { |in=1024, out=1026, cutoff_mod_in=(-1), res_mod_in=(-1), cutoff=1000, resonance=0.5, shape=0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalCutoff = (cutoff.lag(lag) * (1 + cutoffMod)).clip(20, 20000);
    var q = (resonance.lag(lag) + resMod).clip(0, 1).linlin(0, 1, 1, 0.05);
    var vowelFreqs = #[800, 400, 270, 450, 325]; // A E I O U
    var idx = shape.lag(lag) * 4;
    var f1 = Select.kr(idx.round, vowelFreqs) * (finalCutoff / 1000);
    Out.ar(out, BPF.ar(sig, f1.clip(80, 12000), q) * fadein);
}).writeDefFile(dir);
)
