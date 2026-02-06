// Samplers - Buffer playback with variable rate
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

// Sampler - Plays from buffer with variable rate (for scratching)
SynthDef(\imbolc_sampler, { |out=1024, bufnum=0, sliceStart=0, sliceEnd=1,
                             rate=1.0, rate_in=(-1), amp=0.8, loop=0,
                             freq_in=(-1), gate_in=(-1), vel_in=(-1),
                             attack=0.001, decay=0.1, sustain=1.0, release=0.05,
                             amp_mod_in=(-1), srate_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
                            decay_mod_in=(-1), sustain_mod_in=(-1)|
    var rateSig = Select.kr(rate_in >= 0, [rate, In.kr(rate_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var srateMod = Select.kr(srate_mod_in >= 0, [0, In.kr(srate_mod_in)]);
    var attackMod = Select.kr(attack_mod_in >= 0, [0, In.kr(attack_mod_in)]);
    var releaseMod = Select.kr(release_mod_in >= 0, [0, In.kr(release_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var sustainMod = Select.kr(sustain_mod_in >= 0, [0, In.kr(sustain_mod_in)]);
    var finalAmp = amp * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    // If freq_in is provided, calculate rate from pitch (relative to middle C = 60)
    var freqSig = Select.kr(freq_in >= 0, [60.midicps, In.kr(freq_in)]);
    var pitchRate = freqSig / 60.midicps;
    var finalRate = rateSig * pitchRate * (1 + srateMod);
    var numFrames = BufFrames.kr(bufnum);
    var startFrame = sliceStart * numFrames;
    var endFrame = sliceEnd * numFrames;
    var isReverse = finalRate < 0;
    var phasorStart = Select.kr(isReverse, [startFrame, endFrame]);
    var phasorEnd = Select.kr(isReverse, [endFrame, startFrame]);
    var phasorReset = Select.kr(isReverse, [startFrame, endFrame]);
    var phasor = Phasor.ar(
        trig: 0,
        rate: BufRateScale.kr(bufnum) * finalRate,
        start: phasorStart,
        end: phasorEnd,
        resetPos: phasorReset
    );
    var sig = BufRd.ar(2, bufnum, phasor, loop: loop);
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig, doneAction: 2);
    Out.ar(out, sig * finalAmp * env * velSig);
}).writeDefFile(dir);

// Sampler one-shot (for triggering slices without MIDI control)
SynthDef(\imbolc_sampler_oneshot, { |out=1024, bufnum=0, sliceStart=0, sliceEnd=1,
                                      rate=1.0, amp=0.8, gate=1|
    var numFrames = BufFrames.kr(bufnum);
    var startFrame = sliceStart * numFrames;
    var endFrame = sliceEnd * numFrames;

    // Calculate duration to drive Line and Envelope
    var bufRate = BufSampleRate.kr(bufnum) * rate.abs;
    var duration = (endFrame - startFrame).abs / bufRate;

    // Line.ar drives the pointer from start to end over duration
    var isReverse = rate < 0;
    var lineStart = Select.kr(isReverse, [startFrame, endFrame]);
    var lineEnd = Select.kr(isReverse, [endFrame, startFrame]);
    var phasor = Line.ar(lineStart, lineEnd, duration, doneAction: 2);

    var sig = BufRd.ar(2, bufnum, phasor, loop: 0);

    // Simple envelope to de-click
    var attack = 0.002;
    var release = 0.01;
    // Ensure sustain is not negative if duration is very short
    var sustain = (duration - attack - release).max(0);
    var env = EnvGen.kr(Env.linen(attack, sustain, release), doneAction: 2);

    Out.ar(out, sig * amp * env);
}).writeDefFile(dir);

// Time Stretch - Granular playback for independent pitch/speed control (slowed+reverb style)
SynthDef(\imbolc_timestretch, { |out=1024, bufnum=0, sliceStart=0, sliceEnd=1,
    stretch=1.0, pitch=0, grain_size=0.1, overlap=4, amp=0.8,
    freq_in=(-1), gate_in=(-1), vel_in=(-1),
    attack=0.001, decay=0.1, sustain=1.0, release=0.05,
    amp_mod_in=(-1), stretch_mod_in=(-1), pitch_mod_in=(-1), grain_size_mod_in=(-1),
    attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1)|

    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var stretchMod = Select.kr(stretch_mod_in >= 0, [0, In.kr(stretch_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var grainSizeMod = Select.kr(grain_size_mod_in >= 0, [0, In.kr(grain_size_mod_in)]);
    var attackMod = Select.kr(attack_mod_in >= 0, [0, In.kr(attack_mod_in)]);
    var releaseMod = Select.kr(release_mod_in >= 0, [0, In.kr(release_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var sustainMod = Select.kr(sustain_mod_in >= 0, [0, In.kr(sustain_mod_in)]);

    var finalAmp = amp * (1 + ampMod).max(0);
    var finalStretch = (stretch * (1 + stretchMod)).clip(0.1, 10);
    var finalPitch = pitch + (pitchMod * 12);
    var finalGrainSize = (grain_size * (1 + grainSizeMod)).clip(0.01, 1.0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);

    var pitchRatio = 2.pow(finalPitch / 12);
    var numFrames = BufFrames.kr(bufnum);
    var sliceFrames = (sliceEnd - sliceStart) * numFrames;
    var pointerRate = (1 / finalStretch) / (sliceFrames / BufSampleRate.kr(bufnum));
    var pointer = sliceStart + (LFSaw.kr(pointerRate, 1).range(0, 1) * (sliceEnd - sliceStart));

    var sig = Warp1.ar(2, bufnum, pointer, pitchRatio, finalGrainSize, -1, overlap.max(1), 0.0, 4);
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig, doneAction: 2);

    Out.ar(out, sig * finalAmp * env * velSig);
}).writeDefFile(dir);
)
