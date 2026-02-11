// imbolc_organ SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_organ, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, perc=0.5, leslie=0.3,
    d1=1.0, d2=0.8, d3=0.6, d4=0.5, d5=0.4, d6=0.3, d7=0.2, d8=0.1, d9=0.1,
    attack=0.01, decay=0.1, sustain=1.0, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), leslie_mod_in=(-1)|

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
    var leslieMod = Select.kr(leslie_mod_in >= 0, [0, In.kr(leslie_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.005);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalLeslie = (leslie + leslieMod).clip(0, 1);

    // Drawbar harmonic ratios (Hammond style)
    var ratios = [0.5, 1.0, 1.498, 2.0, 2.997, 4.0, 5.04, 5.994, 8.0];
    var drawbars = [d1, d2, d3, d4, d5, d6, d7, d8, d9];

    // Percussion envelope (click attack)
    var percEnv = EnvGen.kr(Env.perc(0.001, 0.15), gateSig) * perc.lag(lag);

    // Sum of harmonic sine waves
    var sig = Mix.fill(9, { |i|
        var harm = SinOsc.ar(finalFreq * ratios[i]);
        harm * drawbars[i].lag(lag);
    });
    var env, lfoRate, lfoAmp, lfoPitch;
    sig = sig / 3; // normalize

    // Add percussion harmonic (3rd harmonic click)
    sig = sig + (SinOsc.ar(finalFreq * 3) * percEnv * 0.3);

    // Main envelope
    env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);

    // Leslie effect (amplitude and pitch modulation)
    lfoRate = 5.5;
    lfoAmp = SinOsc.kr(lfoRate);
    lfoPitch = SinOsc.kr(lfoRate, 0, 0.003 * finalLeslie);
    sig = DelayC.ar(sig, 0.02, lfoPitch.abs * 0.01 + 0.005);
    sig = sig * (1 + (lfoAmp * 0.15 * finalLeslie));

    Out.ar(out, (sig * finalAmp * velSig * env) ! 2);
}).writeDefFile(dir);
)
