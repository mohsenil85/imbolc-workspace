// Synthesis - FM, PhaseMod, Pluck, Formant, Gendy, Chaos, Additive, Wavetable, Granular
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

// FM — 2-operator FM synthesis (DX7 style)
SynthDef(\imbolc_fm, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1), freq=440, amp=0.5, lag=0.02, ratio=2.0, index=1.0, attack=0.01, decay=0.1, sustain=0.7, release=0.3, amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1), index_mod_in=(-1)|
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
    var indexMod = Select.kr(index_mod_in >= 0, [0, In.kr(index_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalIndex = index * (1 + indexMod);
    var modFreq = finalFreq * ratio;
    var modulator = SinOsc.ar(modFreq) * finalIndex * finalFreq;
    var sig = SinOsc.ar(finalFreq + modulator) * finalAmp * velSig;
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);

// PhaseMod — Phase distortion synthesis (Casio CZ style)
SynthDef(\imbolc_phasemod, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1), freq=440, amp=0.5, lag=0.02, ratio=2.0, index=1.0, attack=0.01, decay=0.1, sustain=0.7, release=0.3, amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1), index_mod_in=(-1)|
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
    var indexMod = Select.kr(index_mod_in >= 0, [0, In.kr(index_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalIndex = index * (1 + indexMod);
    var modulator = SinOsc.ar(finalFreq * ratio) * finalIndex;
    var sig = SinOsc.ar(finalFreq, modulator) * finalAmp * velSig;
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);

// Pluck — Karplus-Strong string synthesis
SynthDef(\imbolc_pluck, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1), freq=440, amp=0.5, lag=0.02, decay=2.0, coef=0.3, attack=0.01, sustain=0.7, release=0.3, amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1)|
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
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var maxDelay = 20.reciprocal; // 20 Hz lowest pitch
    var sig = Pluck.ar(WhiteNoise.ar, gateSig, maxDelay, finalFreq.reciprocal, decay, coef) * finalAmp * velSig;
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);

// Formant — Vocal formant synthesis
SynthDef(\imbolc_formant, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1), freq=440, amp=0.5, lag=0.02, formant=800, bw=200, attack=0.01, decay=0.1, sustain=0.7, release=0.3, amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1), formant_mod_in=(-1)|
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
    var formantMod = Select.kr(formant_mod_in >= 0, [0, In.kr(formant_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalFormant = formant * (1 + formantMod);
    var sig = Formant.ar(finalFreq, finalFormant.lag(lag), bw.lag(lag)) * finalAmp * velSig;
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);

// Gendy — Xenakis dynamic stochastic synthesis
SynthDef(\imbolc_gendy, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1), amp=0.5, lag=0.02, ampdist=1, durdist=1, minfreq=100, maxfreq=1000, attack=0.01, decay=0.1, sustain=0.7, release=0.3, amp_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1)|
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var attackMod = Select.kr(attack_mod_in >= 0, [0, In.kr(attack_mod_in)]);
    var releaseMod = Select.kr(release_mod_in >= 0, [0, In.kr(release_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var sustainMod = Select.kr(sustain_mod_in >= 0, [0, In.kr(sustain_mod_in)]);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var sig = Gendy1.ar(ampdist, durdist, 1.0, 1.0, minfreq, maxfreq) * finalAmp * velSig;
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);

// Chaos — Chaotic attractor oscillators (Henon/Lorenz)
SynthDef(\imbolc_chaos, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1), amp=0.5, lag=0.02, model=0, chaos_freq=8000, chaos_param=1.3, attack=0.01, decay=0.1, sustain=0.7, release=0.3, amp_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1), chaos_param_mod_in=(-1)|
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var attackMod = Select.kr(attack_mod_in >= 0, [0, In.kr(attack_mod_in)]);
    var releaseMod = Select.kr(release_mod_in >= 0, [0, In.kr(release_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var sustainMod = Select.kr(sustain_mod_in >= 0, [0, In.kr(sustain_mod_in)]);
    var chaosParamMod = Select.kr(chaos_param_mod_in >= 0, [0, In.kr(chaos_param_mod_in)]);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalChaosParam = chaos_param * (1 + chaosParamMod);
    var henon = HenonN.ar(chaos_freq, finalChaosParam, 0.3);
    var lorenz = LorenzL.ar(chaos_freq, finalChaosParam.linlin(0, 2, 8, 14));
    var sig = Select.ar(model, [henon, lorenz]) * finalAmp * velSig;
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);

// Additive — Harmonic series synthesis
SynthDef(\imbolc_additive, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1), freq=440, amp=0.5, lag=0.02, harmonics=8, rolloff=1.0, attack=0.01, decay=0.1, sustain=0.7, release=0.3, amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1), rolloff_mod_in=(-1)|
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
    var rolloffMod = Select.kr(rolloff_mod_in >= 0, [0, In.kr(rolloff_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalRolloff = (rolloff + rolloffMod).max(0);
    // Build 32 harmonics, gate amplitude on whether index < harmonics
    var sig = Mix.fill(32, { |i|
        var n = i + 1;
        var harmAmp = (n <= harmonics) * (n ** finalRolloff.neg);
        SinOsc.ar(finalFreq * n, 0, harmAmp);
    });
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    sig = sig * finalAmp * velSig;
    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);

// Wavetable — Morphing wavetable oscillator using VOsc
SynthDef(\imbolc_wavetable, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1), freq=440, amp=0.5, lag=0.02, position=0.0, attack=0.01, decay=0.1, sustain=0.7, release=0.3, amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1), position_mod_in=(-1)|
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
    var positionMod = Select.kr(position_mod_in >= 0, [0, In.kr(position_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalPosition = (position + positionMod).clip(0, 1);
    // VOsc morphs between adjacent buffer pairs; position 0-1 maps to bufnum range
    // Expects wavetable buffers loaded at bufnums 100-107 (8 tables)
    var bufpos = finalPosition.linlin(0, 1, 100, 106.99);
    var sig = VOsc.ar(bufpos, finalFreq) * finalAmp * velSig;
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);

// Granular — Pitched granular synthesis using GrainSin (no buffer needed)
SynthDef(\imbolc_granular, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, grain_size=0.05, density=20, spread=0, pitch_rnd=0,
    attack=0.01, decay=0.1, sustain=0.7, release=0.3,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), grain_size_mod_in=(-1), density_mod_in=(-1)|

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
    var grainSizeMod = Select.kr(grain_size_mod_in >= 0, [0, In.kr(grain_size_mod_in)]);
    var densityMod = Select.kr(density_mod_in >= 0, [0, In.kr(density_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalGrainSize = (grain_size * (1 + grainSizeMod)).max(0.001);
    var finalDensity = (density * (1 + densityMod)).max(1);

    var trig = Dust.kr(finalDensity);
    var grainFreq = finalFreq * (2 ** (TRand.kr(-1, 1, trig) * pitch_rnd));
    var pan = TRand.kr(spread.neg, spread, trig);
    var sig = GrainSin.ar(2, trig, finalGrainSize.lag(lag), grainFreq, pan) * finalAmp * velSig;
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, sig * env);
}).writeDefFile(dir);
)
