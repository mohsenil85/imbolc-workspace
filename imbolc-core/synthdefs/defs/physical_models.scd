// Physical Models - Bowed, Blown, Membrane
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

// Bowed — delay-line resonator excited by filtered noise
SynthDef(\imbolc_bowed, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, pressure=0.5, bow_pos=0.12,
    attack=0.01, decay=0.1, sustain=0.7, release=0.3,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), pressure_mod_in=(-1)|

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
    var pressureMod = Select.kr(pressure_mod_in >= 0, [0, In.kr(pressure_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalPressure = (pressure + (pressureMod * 0.5)).clip(0, 1);

    var exciter = BrownNoise.ar * finalPressure.lag(lag) * gateSig;
    var sig = CombL.ar(exciter, 0.05, finalFreq.reciprocal * (1 - bow_pos.lag(lag)),
        finalPressure.lag(lag).linlin(0, 1, 0.5, 4.0));
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    sig = LPF.ar(sig, finalFreq * 4);
    Out.ar(out, (sig * finalAmp * velSig * env) ! 2);
}).writeDefFile(dir);

// Blown — jet noise excitation + waveguide delay
SynthDef(\imbolc_blown, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, pressure=0.5, embouchure=0.5,
    attack=0.01, decay=0.1, sustain=0.7, release=0.3,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), pressure_mod_in=(-1), embouchure_mod_in=(-1)|

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
    var pressureMod = Select.kr(pressure_mod_in >= 0, [0, In.kr(pressure_mod_in)]);
    var embouchureMod = Select.kr(embouchure_mod_in >= 0, [0, In.kr(embouchure_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalPressure = (pressure + (pressureMod * 0.5)).clip(0, 1);
    var finalEmbouchure = (embouchure + (embouchureMod * 0.5)).clip(0, 1);

    var jet = WhiteNoise.ar * finalPressure.lag(lag) * gateSig;
    var embLag = finalEmbouchure.lag(lag);
    var jetClip = (jet * embLag.linlin(0, 1, 1, 4)).clip2(1);
    var sig = CombL.ar(jetClip, 0.05, finalFreq.reciprocal,
        finalPressure.lag(lag).linlin(0, 1, 0.2, 2.0));
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    sig = LPF.ar(sig, finalFreq * (2 + (embLag * 3)));
    Out.ar(out, (sig * finalAmp * velSig * env) ! 2);
}).writeDefFile(dir);

// Membrane — impulse into parallel modal resonators
SynthDef(\imbolc_membrane, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, tension=0.05, loss=0.99,
    attack=0.001, decay=0.3, sustain=0.0, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), tension_mod_in=(-1)|

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
    var tensionMod = Select.kr(tension_mod_in >= 0, [0, In.kr(tension_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalTension = (tension * (1 + tensionMod)).clip(0.001, 1);

    var exciter = WhiteNoise.ar(0.5) * EnvGen.ar(Env.perc(0.001, finalTension.lag(lag)), gateSig);
    var mode1 = CombL.ar(exciter, 0.05, finalFreq.reciprocal, loss.lag(lag).linlin(0.9, 1.0, 0.1, 3.0));
    var mode2 = CombL.ar(exciter, 0.05, (finalFreq * 1.59).reciprocal, loss.lag(lag).linlin(0.9, 1.0, 0.05, 2.0));
    var mode3 = CombL.ar(exciter, 0.05, (finalFreq * 2.14).reciprocal, loss.lag(lag).linlin(0.9, 1.0, 0.02, 1.5));
    var sig = mode1 + (mode2 * 0.5) + (mode3 * 0.25);
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, (sig * finalAmp * velSig * env) ! 2);
}).writeDefFile(dir);
)
