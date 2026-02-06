// imbolc_marimba SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_marimba, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.6, lag=0.02, hardness=0.5, bar_pos=0.2,
    attack=0.001, decay=1.5, sustain=0.0, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), hardness_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var hardnessMod = Select.kr(hardness_mod_in >= 0, [0, In.kr(hardness_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01);
    var finalHardness = (hardness + hardnessMod).clip(0, 1);
    var trig = Trig1.kr(gateSig, 0.001);
    var modeRatios = [1, 2.57, 4.53, 5.41, 6.59];
    var modeAmps = [1, 0.4, 0.2, 0.15, 0.1];
    var exciteTime = (1 - finalHardness) * 0.01 + 0.001;
    var exciter = WhiteNoise.ar * EnvGen.ar(Env.perc(0.0001, exciteTime), trig);
    var sig = Mix.fill(5, { |i|
        var modeFreq = finalFreq * modeRatios[i];
        var modeDecay = finalDecay * [1, 0.7, 0.5, 0.4, 0.3][i];
        BPF.ar(exciter, modeFreq.clip(20, 18000), 0.001) * modeAmps[i] *
            EnvGen.kr(Env.perc(0.001, modeDecay), trig);
    }) * 30;
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
