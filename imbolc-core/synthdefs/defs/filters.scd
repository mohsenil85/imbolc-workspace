// Filters - Read from audio bus, optional cutoff modulation, write to audio bus
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

SynthDef(\imbolc_lpf, { |in=1024, out=1026, cutoff_mod_in=(-1), res_mod_in=(-1), cutoff=1000, resonance=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalCutoff = (cutoff.lag(lag) * (1 + cutoffMod)).clip(20, 20000);
    var finalRes = (resonance.lag(lag) + resMod).clip(0, 1);
    var q = finalRes.linlin(0, 1, 1, 0.1);
    Out.ar(out, RLPF.ar(sig, finalCutoff, q) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_hpf, { |in=1024, out=1026, cutoff_mod_in=(-1), res_mod_in=(-1), cutoff=1000, resonance=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalCutoff = (cutoff.lag(lag) * (1 + cutoffMod)).clip(20, 20000);
    var finalRes = (resonance.lag(lag) + resMod).clip(0, 1);
    var q = finalRes.linlin(0, 1, 1, 0.1);
    Out.ar(out, RHPF.ar(sig, finalCutoff, q) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_bpf, { |in=1024, out=1026, cutoff_mod_in=(-1), res_mod_in=(-1), cutoff=1000, resonance=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalCutoff = (cutoff.lag(lag) * (1 + cutoffMod)).clip(20, 20000);
    var finalRes = (resonance.lag(lag) + resMod).clip(0, 1);
    var q = finalRes.linlin(0, 1, 1, 0.01);
    Out.ar(out, BPF.ar(sig, finalCutoff, q) * fadein);
}).writeDefFile(dir);

// Notch (Band-Reject)
SynthDef(\imbolc_notch, { |in=1024, out=1026, cutoff_mod_in=(-1), res_mod_in=(-1), cutoff=1000, resonance=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalCutoff = (cutoff.lag(lag) * (1 + cutoffMod)).clip(20, 20000);
    var q = (resonance.lag(lag) + resMod).clip(0, 1).linlin(0, 1, 1, 0.01);
    Out.ar(out, BRF.ar(sig, finalCutoff, q) * fadein);
}).writeDefFile(dir);

// Comb — cutoff maps to comb frequency, resonance to decay
SynthDef(\imbolc_comb, { |in=1024, out=1026, cutoff_mod_in=(-1), res_mod_in=(-1), cutoff=440, resonance=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalCutoff = (cutoff.lag(lag) * (1 + cutoffMod)).clip(20, 20000);
    var finalRes = (resonance.lag(lag) + resMod).clip(0, 1);
    var delay = finalCutoff.reciprocal.clip(0.00005, 0.05);
    var decayTime = finalRes.linlin(0, 1, 0.01, 2.0);
    Out.ar(out, CombC.ar(sig, 0.05, delay, decayTime) * fadein);
}).writeDefFile(dir);

// Allpass — same param mapping as Comb
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

// Vowel — morphs A/E/I/O/U formants via shape param
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

// ResDrive — resonant LPF with input saturation
SynthDef(\imbolc_resdrive, { |in=1024, out=1026, cutoff_mod_in=(-1), res_mod_in=(-1), cutoff=1000, resonance=0.5, drive=1, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalCutoff = (cutoff.lag(lag) * (1 + cutoffMod)).clip(20, 20000);
    var q = (resonance.lag(lag) + resMod).clip(0, 1).linlin(0, 1, 1, 0.05);
    Out.ar(out, RLPF.ar((sig * drive.lag(lag)).tanh, finalCutoff, q) * fadein);
}).writeDefFile(dir);
)
