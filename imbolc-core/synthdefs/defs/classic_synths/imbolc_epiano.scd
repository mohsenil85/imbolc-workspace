// imbolc_epiano SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_epiano, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, tine=1.0, bark=0.3, tremolo=0.2,
    attack=0.001, decay=2.0, sustain=0.3, release=0.3,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), tine_mod_in=(-1)|

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
    var tineMod = Select.kr(tine_mod_in >= 0, [0, In.kr(tine_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalTine = (tine + tineMod).clip(0, 2);

    // Velocity-sensitive modulation
    var modIndex = velSig.linlin(0, 1, 1, 4) * finalTine;

    // Modulator with fast decay
    var modEnv = EnvGen.kr(Env.perc(0.001, 0.3), gateSig);
    var modulator = SinOsc.ar(finalFreq * 14) * modEnv * modIndex;

    // Carrier
    var carrier = SinOsc.ar(finalFreq + modulator);

    // "Bark" - slight distortion and harmonic content
    var barkSig = SinOsc.ar(finalFreq * 7) * bark.lag(lag) * 0.1 * modEnv;

    // Main envelope
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);

    // Tremolo
    var trem = 1 + (SinOsc.kr(4.5) * tremolo.lag(lag) * 0.1);

    var sig = (carrier + barkSig) * env * trem;
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
