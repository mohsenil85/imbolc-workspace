// imbolc_spectrum SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_spectrum, {
    var sig = In.ar(0, 2);
    var mono = (sig[0] + sig[1]) * 0.5;
    var freqs = #[60, 150, 400, 1000, 2500, 6000, 15000];
    var bws = #[0.8, 0.7, 0.6, 0.5, 0.5, 0.5, 0.5]; // wider bandwidth at low freqs
    var amps = Array.fill(7, { |i|
        Amplitude.kr(BPF.ar(mono, freqs[i], bws[i]), 0.05, 0.2);
    });
    SendReply.kr(Impulse.kr(20), '/spectrum', amps);
}).writeDefFile(dir);
)
