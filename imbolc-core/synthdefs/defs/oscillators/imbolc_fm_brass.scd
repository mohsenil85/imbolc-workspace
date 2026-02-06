// imbolc_fm_brass SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_fm_brass, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.6, lag=0.02, brightness=0.6,
    attack=0.05, decay=0.2, sustain=0.7, release=0.3,
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
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalBrightness = (brightness + brightnessMod).clip(0, 1);

    // Ratio of 1:1 gives odd harmonics (brass-like)
    var ratio = 1.0;

    // Attack-sensitive modulation index
    var modEnv = EnvGen.kr(Env.adsr(finalAttack * 0.5, finalDecay, finalSustain * 0.5, finalRelease * 0.5), gateSig);
    var index = velSig.linlin(0, 1, 2, 8) * finalBrightness * modEnv;

    // Simple 2-op FM
    var modulator = SinOsc.ar(finalFreq * ratio) * index * finalFreq;
    var carrier = SinOsc.ar(finalFreq + modulator);

    // Add slight second voice for thickness
    var mod2 = SinOsc.ar(finalFreq * 1.003 * ratio) * index * finalFreq * 0.8;
    var carrier2 = SinOsc.ar(finalFreq * 1.003 + mod2) * 0.5;

    var sig = carrier + carrier2;

    // Amp envelope
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);

    Out.ar(out, (sig * finalAmp * velSig * env * 0.6) ! 2);
}).writeDefFile(dir);
)
