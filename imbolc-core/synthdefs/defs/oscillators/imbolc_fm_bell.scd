// imbolc_fm_bell SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_fm_bell, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, brightness=0.7,
    attack=0.001, decay=4.0, sustain=0.0, release=0.5,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), brightness_mod_in=(-1)|

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
    var brightnessMod = Select.kr(brightness_mod_in >= 0, [0, In.kr(brightness_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.005);
    var finalRelease = (release * (1 + releaseMod)).max(0.005);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalBrightness = (brightness + brightnessMod).clip(0, 1);

    var trig = Trig1.kr(gateSig, 0.001);

    // Inharmonic ratios for bell character
    var ratio1 = 3.5;  // modulator 1 ratio
    var ratio2 = 7.1;  // modulator 2 ratio

    // Modulation indices decay over time
    var modEnv1 = EnvGen.kr(Env.perc(finalAttack, finalDecay * 0.3), trig);
    var modEnv2 = EnvGen.kr(Env.perc(finalAttack, finalDecay * 0.5), trig);

    var index1 = velSig.linlin(0, 1, 2, 6) * finalBrightness * modEnv1;
    var index2 = velSig.linlin(0, 1, 1, 4) * finalBrightness * modEnv2;

    // Modulator 2 -> Modulator 1 -> Carrier
    var mod2 = SinOsc.ar(finalFreq * ratio2) * index2 * finalFreq;
    var mod1 = SinOsc.ar(finalFreq * ratio1 + mod2) * index1 * finalFreq;
    var carrier = SinOsc.ar(finalFreq + mod1);

    // Amp envelope
    var env = EnvGen.ar(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig, doneAction: 14);

    Out.ar(out, (carrier * finalAmp * velSig * env) ! 2);
}).writeDefFile(dir);
)
