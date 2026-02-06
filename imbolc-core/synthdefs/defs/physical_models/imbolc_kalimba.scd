// imbolc_kalimba SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_kalimba, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, damping=0.3, brightness=0.5,
    attack=0.001, decay=2.0, sustain=0.0, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod) * (1 - damping.lag(lag) * 0.8)).max(0.01);
    var trig = Trig1.kr(gateSig, 0.001);
    var modeRatios = [1, 2.8, 5.4, 8.93];
    var brightLag = brightness.lag(lag);
    var exciter = Impulse.ar(0) * EnvGen.kr(Env.perc(0.0001, 0.005), trig);
    var sig = Mix.fill(4, { |i|
        var modeFreq = finalFreq * modeRatios[i];
        var ringTime = finalDecay / (i + 1);
        var modeAmp = [1, 0.3, 0.15, 0.08][i] * [1, brightLag, brightLag * 0.7, brightLag * 0.5][i];
        Ringz.ar(exciter, modeFreq.clip(20, 18000), ringTime) * modeAmp;
    }) * 0.3;
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
