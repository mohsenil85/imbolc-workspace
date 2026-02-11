// imbolc_acid SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_acid, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.6, lag=0.02, cutoff=800, res=0.7, accent=0.5, envmod=0.5,
    attack=0.001, decay=0.2, sustain=0.0, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), cutoff_mod_in=(-1), res_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(0.05), In.kr(freq_in)]); // slide lag
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var attackMod = Select.kr(attack_mod_in >= 0, [0, In.kr(attack_mod_in)]);
    var releaseMod = Select.kr(release_mod_in >= 0, [0, In.kr(release_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var sustainMod = Select.kr(sustain_mod_in >= 0, [0, In.kr(sustain_mod_in)]);
    var cutoffMod = Select.kr(cutoff_mod_in >= 0, [0, In.kr(cutoff_mod_in)]);
    var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.005);
    var finalRelease = (release * (1 + releaseMod)).max(0.005);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalCutoff = cutoff * (2 ** cutoffMod);
    var finalRes = (res + resMod).clip(0, 0.99);

    // Accent affects filter and amp
    var accentLevel = velSig.linlin(0.5, 1.0, 0, 1) * accent.lag(lag);

    // Saw oscillator with slight sub
    var osc = Saw.ar(finalFreq) + (Pulse.ar(finalFreq * 0.5, 0.5) * 0.3);

    // Filter envelope
    var filterEnv = EnvGen.kr(Env.perc(0.001, finalDecay), gateSig);
    var envAmount = envmod.lag(lag) * (1 + accentLevel);
    var filt = finalCutoff * (1 + (filterEnv * envAmount * 8));
    var sig = RLPF.ar(osc, filt.clip(20, 18000), 1 - finalRes);

    // Amp envelope
    var env = EnvGen.ar(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig, doneAction: 14);

    // Accent boost
    var accentAmp = 1 + (accentLevel * 0.5);

    // Slight saturation
    sig = (sig * 1.5).tanh;

    Out.ar(out, (sig * finalAmp * velSig * env * accentAmp) ! 2);
}).writeDefFile(dir);
)
