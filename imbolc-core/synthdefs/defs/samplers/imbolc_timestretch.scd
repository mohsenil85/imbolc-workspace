// imbolc_timestretch SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_timestretch, { |out=1024, bufnum=0, sliceStart=0, sliceEnd=1,
    stretch=1.0, pitch=0, grain_size=0.1, overlap=4, amp=0.8, lag=0.02,
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

    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalStretch = (stretch.lag(lag) * (1 + stretchMod)).clip(0.1, 10);
    var finalPitch = pitch.lag(lag) + (pitchMod * 12);
    var finalGrainSize = (grain_size.lag(lag) * (1 + grainSizeMod)).clip(0.01, 1.0);
    var finalAttack = (attack * (1 + attackMod)).max(0.005);
    var finalRelease = (release * (1 + releaseMod)).max(0.005);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);

    var pitchRatio = 2.pow(finalPitch / 12);
    var numFrames = BufFrames.kr(bufnum);
    var sliceFrames = (sliceEnd - sliceStart) * numFrames;
    var pointerRate = (1 / finalStretch) / (sliceFrames / BufSampleRate.kr(bufnum));
    var pointer = sliceStart + (LFSaw.kr(pointerRate, 1).range(0, 1) * (sliceEnd - sliceStart));

    var sig = LeakDC.ar(Warp1.ar(2, bufnum, pointer, pitchRatio, finalGrainSize, -1, overlap.max(1), 0.0, 4));
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig, doneAction: 2);

    Out.ar(out, sig * finalAmp * env * velSig);
}).writeDefFile(dir);
)
