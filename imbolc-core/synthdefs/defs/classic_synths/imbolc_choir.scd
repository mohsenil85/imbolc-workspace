// imbolc_choir SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_choir, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, vowel=0.5, spread=0.02, vibrato=0.3,
    attack=0.3, decay=0.2, sustain=0.8, release=0.4,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), vowel_mod_in=(-1)|

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
    var vowelMod = Select.kr(vowel_mod_in >= 0, [0, In.kr(vowel_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalSpread = spread * (1 + (detuneMod * 0.5)).max(0);
    var finalVowel = (vowel + (vowelMod * 0.5)).clip(0, 1);

    // Vibrato LFO
    var vib = SinOsc.kr(5) * vibrato.lag(lag) * 0.003 * finalFreq;

    // 4 detuned saws
    var saws = Array.fill(4, { |i|
        var dt = finalSpread * (i - 1.5) / 1.5;
        Saw.ar((finalFreq + vib) * (1 + dt));
    });
    var mix = saws.sum * 0.25;

    // Formant filtering - interpolate between vowels (A-E-I-O-U approximation)
    var formant1 = LinXFade2.ar(
        BPF.ar(mix, 800, 0.2),
        BPF.ar(mix, 400, 0.15),
        (finalVowel * 4 - 1).clip(-1, 1)
    );
    var formant2 = LinXFade2.ar(
        BPF.ar(mix, 1200, 0.25),
        BPF.ar(mix, 2300, 0.2),
        (finalVowel * 4 - 2).clip(-1, 1)
    );
    var sig = (formant1 * 0.6) + (formant2 * 0.4) + (mix * 0.3);

    // Slow attack envelope
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);

    // Subtle chorus
    var chorus = DelayC.ar(sig, 0.05, SinOsc.kr(0.5, 0, 0.003, 0.01));
    sig = (sig + chorus) * 0.6;

    Out.ar(out, (sig * finalAmp * velSig * env) ! 2);
}).writeDefFile(dir);
)
