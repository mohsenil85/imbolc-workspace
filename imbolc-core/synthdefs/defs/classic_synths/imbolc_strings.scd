// imbolc_strings SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_strings, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, spread=0.015, vibrato=0.2,
    attack=0.4, decay=0.3, sustain=0.8, release=0.5,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var attackMod = Select.kr(attack_mod_in >= 0, [0, In.kr(attack_mod_in)]);
    var releaseMod = Select.kr(release_mod_in >= 0, [0, In.kr(release_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var sustainMod = Select.kr(sustain_mod_in >= 0, [0, In.kr(sustain_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.005);
    var finalRelease = (release * (1 + releaseMod)).max(0.005);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalSpread = spread * (1 + detuneMod).max(0);

    // Delayed vibrato (comes in after attack)
    var vibDelay = EnvGen.kr(Env([0, 0, 1], [0.5, 0.3]));
    var vib = SinOsc.kr(5.5) * vibrato.lag(lag) * 0.004 * finalFreq * vibDelay;

    // Multiple detuned saws for ensemble effect
    var saws = Array.fill(6, { |i|
        var dt = finalSpread * (i - 2.5) / 2.5;
        var f = (finalFreq + vib) * (1 + dt);
        Saw.ar(f);
    });
    var sig = saws.sum / 6;
    var env, left, right;

    // Gentle low-pass to soften
    sig = LPF.ar(sig, 5000);

    // Slow attack envelope
    env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);

    // Stereo spread
    left = DelayC.ar(sig, 0.02, 0.007);
    right = DelayC.ar(sig, 0.02, 0.012);

    Out.ar(out, [left, right] * finalAmp * velSig * env);
}).writeDefFile(dir);
)
