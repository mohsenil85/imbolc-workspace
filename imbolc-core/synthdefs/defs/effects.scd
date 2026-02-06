// Effects - Audio in/out with bus routing
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

SynthDef(\imbolc_delay, { |in=1024, out=1026, time=0.3, feedback=0.5, mix=0.3, lag=0.02, time_mod_in=(-1), feedback_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var timeMod = Select.kr(time_mod_in >= 0, [0, In.kr(time_mod_in)]);
    var fbMod = Select.kr(feedback_mod_in >= 0, [0, In.kr(feedback_mod_in)]);
    var finalTime = (time.lag(lag) * (1 + timeMod)).clip(0.001, 2.0);
    var finalFb = (feedback.lag(lag) + fbMod).clip(0, 1);
    var delayed = CombL.ar(sig, 2.0, finalTime, finalFb * 4);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (delayed * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_reverb, { |in=1024, out=1026, room=0.5, damp=0.5, mix=0.3, lag=0.02, mix_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mixMod = Select.kr(mix_mod_in >= 0, [0, In.kr(mix_mod_in)]);
    var finalMix = (mix.lag(lag) + mixMod).clip(0, 1);
    var wet = FreeVerb2.ar(sig[0], sig[1], finalMix, room.lag(lag), damp.lag(lag));
    Out.ar(out, wet * fadein);
}).writeDefFile(dir);

// Gate/Tremolo effect
// shape: 0=sine (tremolo), 1=square (hard gate), 2=saw (ramp)
SynthDef(\imbolc_gate, { |in=1024, out=1026, rate=4, depth=1, shape=1, lag=0.02, rate_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var rateMod = Select.kr(rate_mod_in >= 0, [0, In.kr(rate_mod_in)]);
    var finalRate = (rate.lag(lag) * (1 + rateMod)).max(0.1);
    var depthLag = depth.lag(lag);
    var sine = SinOsc.kr(finalRate).range(1 - depthLag, 1);
    var square = LFPulse.kr(finalRate, width: 0.5).range(1 - depthLag, 1);
    var saw = LFSaw.kr(finalRate).range(1 - depthLag, 1);
    var lfo = Select.kr(shape, [sine, square, saw]);
    Out.ar(out, sig * lfo * fadein);
}).writeDefFile(dir);

// Tape Compressor - warm analog-style compression with saturation
SynthDef(\imbolc_tape_comp, { |in=1024, out=1026, drive=1.5, threshold=0.5, ratio=3, makeup=1.0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var driven = (sig * drive.lag(lag)).tanh;
    var compressed = Compander.ar(driven, driven,
        thresh: threshold,
        slopeBelow: 1.0,
        slopeAbove: ratio.reciprocal,
        clampTime: 0.01,
        relaxTime: 0.1
    );
    var output = compressed * makeup.lag(lag);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (output * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Sidechain Compressor - driven by external bus signal
SynthDef(\imbolc_sc_comp, { |in=1024, out=1026, sidechain_in=0, threshold=0.3, ratio=4, attack=0.01, release=0.1, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var scSig = Select.ar(sidechain_in > 0, [sig, In.ar(sidechain_in, 2)]);
    var compressed = Compander.ar(sig, scSig,
        thresh: threshold,
        slopeBelow: 1.0,
        slopeAbove: ratio.reciprocal,
        clampTime: attack,
        relaxTime: release
    );
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (compressed * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Chorus — DelayL + SinOsc modulation
SynthDef(\imbolc_chorus, { |in=1024, out=1026, rate=0.5, depth=0.005, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var modL = SinOsc.kr(rate, 0) * depth;
    var modR = SinOsc.kr(rate, 0.5pi) * depth;
    var wet = [
        DelayL.ar(sig[0], 0.05, (depth + modL).clip(0.0001, 0.05)),
        DelayL.ar(sig[1], 0.05, (depth + modR).clip(0.0001, 0.05))
    ];
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Flanger — CombL with short modulated delay
SynthDef(\imbolc_flanger, { |in=1024, out=1026, rate=0.3, depth=0.003, feedback=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mod = SinOsc.kr(rate) * depth;
    var delayTime = (depth + mod).clip(0.0001, 0.01);
    var wet = CombL.ar(sig, 0.01, delayTime, feedback.lag(lag) * 0.5);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Phaser — AllpassL chain with LFO sweep
SynthDef(\imbolc_phaser, { |in=1024, out=1026, rate=0.5, depth=0.5, stages=4, feedback=0.3, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var lfo = SinOsc.kr(rate).range(200, 4000);
    var wet = sig;
    // 6 allpass stages, gated by stages param
    6.do { |i|
        var active = (i < stages);
        var freq = lfo * ((i + 1) / 6);
        var delay = freq.reciprocal.clip(0.00001, 0.01);
        wet = Select.ar(active, [wet, AllpassL.ar(wet, 0.01, delay, 0.001)]);
    };
    wet = wet + (LocalIn.ar(2) * feedback.lag(lag));
    LocalOut.ar(wet);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Tremolo — Amplitude modulation, shape-selectable
// shape: 0=sine, 1=square, 2=saw
SynthDef(\imbolc_tremolo, { |in=1024, out=1026, rate=4, depth=0.5, shape=0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var sine = SinOsc.kr(rate).range(1 - depth, 1);
    var square = LFPulse.kr(rate, width: 0.5).range(1 - depth, 1);
    var saw = LFSaw.kr(rate).range(1 - depth, 1);
    var lfo = Select.kr(shape, [sine, square, saw]);
    var wet = sig * lfo;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Distortion — hard clip / soft clip / foldback
// mode: 0=hard clip, 1=soft clip (tanh), 2=foldback
SynthDef(\imbolc_distortion, { |in=1024, out=1026, drive=2, mode=0, tone=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var driven = sig * drive.lag(lag);
    var hard = driven.clip2(1);
    var soft = driven.tanh;
    var fold = driven.fold2(1);
    var distorted = Select.ar(mode, [hard, soft, fold]);
    var toneFreq = tone.lag(lag).linexp(0, 1, 300, 12000);
    var filtered = LPF.ar(distorted, toneFreq);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (filtered * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Bitcrusher — Latch + bit reduction
SynthDef(\imbolc_bitcrusher, { |in=1024, out=1026, rate=8000, bits=8, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var crushed = Latch.ar(sig, Impulse.ar(rate));
    var bitDepth = (2 ** bits);
    crushed = (crushed * bitDepth).round / bitDepth;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (crushed * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Wavefolder — fold2 with drive
SynthDef(\imbolc_wavefolder, { |in=1024, out=1026, drive=1, symmetry=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var offset = (symmetry.lag(lag) - 0.5) * 0.5;
    var driven = (sig + offset) * drive.lag(lag);
    var folded = driven.fold2(1);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (folded * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Saturator — tanh saturation + tone filter
SynthDef(\imbolc_saturator, { |in=1024, out=1026, drive=1.5, color=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var saturated = (sig * drive.lag(lag)).tanh;
    var toneFreq = color.lag(lag).linexp(0, 1, 500, 15000);
    var filtered = LPF.ar(saturated, toneFreq);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (filtered * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Tilt EQ — BLowShelf + BHiShelf complementary
SynthDef(\imbolc_tilt_eq, { |in=1024, out=1026, tilt=0, frequency=1000, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var tiltDb = tilt.lag(lag) * 12; // -12 to +12 dB
    var lo = BLowShelf.ar(sig, frequency.lag(lag), 1, tiltDb);
    var result = BHiShelf.ar(lo, frequency.lag(lag), 1, tiltDb.neg);
    Out.ar(out, result * fadein);
}).writeDefFile(dir);

// Stereo Widener — Mid/Side processing
SynthDef(\imbolc_stereo_widener, { |in=1024, out=1026, width=1, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mid = (sig[0] + sig[1]) * 0.5;
    var side = (sig[0] - sig[1]) * 0.5;
    var widthLag = width.lag(lag);
    var wet = [mid + (side * widthLag), mid - (side * widthLag)];
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Frequency Shifter — FreqShift UGen
SynthDef(\imbolc_freq_shifter, { |in=1024, out=1026, shift_hz=0, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var wet = FreqShift.ar(sig, shift_hz.lag(lag));
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Limiter — Limiter UGen
SynthDef(\imbolc_limiter, { |in=1024, out=1026, threshold=0.9, release=0.01, ceiling=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var limited = Limiter.ar(sig, threshold.lag(lag), release.lag(lag));
    Out.ar(out, limited * ceiling.lag(lag) * fadein);
}).writeDefFile(dir);

// Pitch Shifter — PitchShift UGen
SynthDef(\imbolc_pitch_shifter, { |in=1024, out=1026, shift=0, window=0.2, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var ratio = 2 ** (shift.lag(lag) / 12); // semitones to ratio
    var wet = PitchShift.ar(sig, window.lag(lag), ratio);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Vinyl — DelayL wow/flutter + Dust crackle + HPF hiss
SynthDef(\imbolc_vinyl, { |in=1024, out=1026, wow=0.3, flutter=0.3, noise=0.1, hiss=0.05, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    // Wow (slow pitch modulation) and flutter (fast pitch modulation)
    var wowMod = SinOsc.kr(0.3 + (wow * 0.5)) * wow * 0.003;
    var flutterMod = SinOsc.kr(6 + (flutter * 4)) * flutter * 0.001;
    var totalMod = wowMod + flutterMod;
    var modulated = DelayL.ar(sig, 0.05, (0.005 + totalMod).clip(0.0001, 0.05));
    // Crackle noise (Dust)
    var crackle = Dust.ar(3 + (noise * 20)) * noise * 0.15;
    // Hiss (filtered noise)
    var hissNoise = HPF.ar(WhiteNoise.ar(hiss * 0.05), 5000);
    var wet = modulated + crackle + hissNoise;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Cabinet — Shaped LPF profiles + mild saturation
// type: 0=small combo, 1=large combo, 2=4x12 stack, 3=open back
SynthDef(\imbolc_cabinet, { |in=1024, out=1026, type=0, tone=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var toneFreq = tone.lag(lag);
    // Different cabinet profiles with different LPF cutoffs and resonance
    var small = LPF.ar(sig.tanh, toneFreq.linexp(0, 1, 800, 5000));
    var large = LPF.ar(sig.tanh, toneFreq.linexp(0, 1, 600, 4000));
    var stack = LPF.ar(BPeakEQ.ar(sig.tanh, 2000, 1, 3), toneFreq.linexp(0, 1, 500, 3500));
    var open = LPF.ar(sig.tanh, toneFreq.linexp(0, 1, 1000, 6000));
    var wet = Select.ar(type, [small, large, stack, open]);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Granular Delay — BufWr + TGrains into LocalBuf
SynthDef(\imbolc_granular_delay, { |in=1024, out=1026, time=0.3, grain_size=0.1, density=10, pitch=0, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var bufFrames = 44100 * 2; // 2 second buffer
    var buf = LocalBuf(bufFrames, 1);
    var phase = Phasor.ar(0, 1, 0, bufFrames);
    var readPos, pitchRatio, trig, wet;
    BufWr.ar(mono, buf, phase);
    readPos = (phase - (time.lag(lag) * SampleRate.ir)).wrap(0, bufFrames);
    pitchRatio = 2 ** (pitch.lag(lag) / 12);
    trig = Impulse.ar(density);
    wet = TGrains.ar(2, trig, buf, pitchRatio, readPos / SampleRate.ir, grain_size.lag(lag), 0, 0.5);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Granular Freeze — Frozen grain position + scatter
SynthDef(\imbolc_granular_freeze, { |in=1024, out=1026, grain_size=0.1, density=10, pitch=0, spread=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var bufFrames = 44100 * 2;
    var buf = LocalBuf(bufFrames, 1);
    var phase = Phasor.ar(0, 1, 0, bufFrames);
    var pitchRatio, trig, scatter, grainPos, wet;
    BufWr.ar(mono, buf, phase);
    pitchRatio = 2 ** (pitch.lag(lag) / 12);
    trig = Impulse.ar(density);
    // Scatter grain position around current write position
    scatter = TRand.ar(0, spread.lag(lag) * bufFrames, trig);
    grainPos = (phase - scatter).wrap(0, bufFrames) / SampleRate.ir;
    wet = TGrains.ar(2, trig, buf, pitchRatio, grainPos, grain_size.lag(lag), 0, 0.5);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// ============================================================================
// Mono effect variants - read/write single channel
// ============================================================================

SynthDef(\imbolc_delay_mono, { |in=1024, out=1026, time=0.3, feedback=0.5, mix=0.3, lag=0.02, time_mod_in=(-1), feedback_mod_in=(-1)|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var timeMod = Select.kr(time_mod_in >= 0, [0, In.kr(time_mod_in)]);
    var fbMod = Select.kr(feedback_mod_in >= 0, [0, In.kr(feedback_mod_in)]);
    var finalTime = (time.lag(lag) * (1 + timeMod)).clip(0.001, 2.0);
    var finalFb = (feedback.lag(lag) + fbMod).clip(0, 1);
    var delayed = CombL.ar(sig, 2.0, finalTime, finalFb * 4);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (delayed * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_reverb_mono, { |in=1024, out=1026, room=0.5, damp=0.5, mix=0.3, lag=0.02, mix_mod_in=(-1)|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var mixMod = Select.kr(mix_mod_in >= 0, [0, In.kr(mix_mod_in)]);
    var finalMix = (mix.lag(lag) + mixMod).clip(0, 1);
    var wet = FreeVerb.ar(sig, finalMix, room.lag(lag), damp.lag(lag));
    Out.ar(out, wet * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_gate_mono, { |in=1024, out=1026, rate=4, depth=1, shape=1, lag=0.02, rate_mod_in=(-1)|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var rateMod = Select.kr(rate_mod_in >= 0, [0, In.kr(rate_mod_in)]);
    var finalRate = (rate.lag(lag) * (1 + rateMod)).max(0.1);
    var depthLag = depth.lag(lag);
    var sine = SinOsc.kr(finalRate).range(1 - depthLag, 1);
    var square = LFPulse.kr(finalRate, width: 0.5).range(1 - depthLag, 1);
    var saw = LFSaw.kr(finalRate).range(1 - depthLag, 1);
    var lfo = Select.kr(shape, [sine, square, saw]);
    Out.ar(out, sig * lfo * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_tape_comp_mono, { |in=1024, out=1026, drive=1.5, threshold=0.5, ratio=3, makeup=1.0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var driven = (sig * drive.lag(lag)).tanh;
    var compressed = Compander.ar(driven, driven,
        thresh: threshold,
        slopeBelow: 1.0,
        slopeAbove: ratio.reciprocal,
        clampTime: 0.01,
        relaxTime: 0.1
    );
    var output = compressed * makeup.lag(lag);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (output * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_sc_comp_mono, { |in=1024, out=1026, sidechain_in=0, threshold=0.3, ratio=4, attack=0.01, release=0.1, mix=1.0, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var scSig = Select.ar(sidechain_in > 0, [sig, In.ar(sidechain_in, 1)]);
    var compressed = Compander.ar(sig, scSig,
        thresh: threshold,
        slopeBelow: 1.0,
        slopeAbove: ratio.reciprocal,
        clampTime: attack,
        relaxTime: release
    );
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (compressed * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_chorus_mono, { |in=1024, out=1026, rate=0.5, depth=0.005, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var mod = SinOsc.kr(rate, 0) * depth;
    var wet = DelayL.ar(sig, 0.05, (depth + mod).clip(0.0001, 0.05));
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_flanger_mono, { |in=1024, out=1026, rate=0.3, depth=0.003, feedback=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var mod = SinOsc.kr(rate) * depth;
    var delayTime = (depth + mod).clip(0.0001, 0.01);
    var wet = CombL.ar(sig, 0.01, delayTime, feedback.lag(lag) * 0.5);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_phaser_mono, { |in=1024, out=1026, rate=0.5, depth=0.5, stages=4, feedback=0.3, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var lfo = SinOsc.kr(rate).range(200, 4000);
    var wet = sig;
    6.do { |i|
        var active = (i < stages);
        var freq = lfo * ((i + 1) / 6);
        var delay = freq.reciprocal.clip(0.00001, 0.01);
        wet = Select.ar(active, [wet, AllpassL.ar(wet, 0.01, delay, 0.001)]);
    };
    wet = wet + (LocalIn.ar(1) * feedback.lag(lag));
    LocalOut.ar(wet);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_tremolo_mono, { |in=1024, out=1026, rate=4, depth=0.5, shape=0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var sine = SinOsc.kr(rate).range(1 - depth, 1);
    var square = LFPulse.kr(rate, width: 0.5).range(1 - depth, 1);
    var saw = LFSaw.kr(rate).range(1 - depth, 1);
    var lfo = Select.kr(shape, [sine, square, saw]);
    var wet = sig * lfo;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_distortion_mono, { |in=1024, out=1026, drive=2, mode=0, tone=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var driven = sig * drive.lag(lag);
    var hard = driven.clip2(1);
    var soft = driven.tanh;
    var fold = driven.fold2(1);
    var distorted = Select.ar(mode, [hard, soft, fold]);
    var toneFreq = tone.lag(lag).linexp(0, 1, 300, 12000);
    var filtered = LPF.ar(distorted, toneFreq);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (filtered * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_bitcrusher_mono, { |in=1024, out=1026, rate=8000, bits=8, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var crushed = Latch.ar(sig, Impulse.ar(rate));
    var bitDepth = (2 ** bits);
    crushed = (crushed * bitDepth).round / bitDepth;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (crushed * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_wavefolder_mono, { |in=1024, out=1026, drive=1, symmetry=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var offset = (symmetry.lag(lag) - 0.5) * 0.5;
    var driven = (sig + offset) * drive.lag(lag);
    var folded = driven.fold2(1);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (folded * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_saturator_mono, { |in=1024, out=1026, drive=1.5, color=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var saturated = (sig * drive.lag(lag)).tanh;
    var toneFreq = color.lag(lag).linexp(0, 1, 500, 15000);
    var filtered = LPF.ar(saturated, toneFreq);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (filtered * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_tilt_eq_mono, { |in=1024, out=1026, tilt=0, frequency=1000, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var tiltDb = tilt.lag(lag) * 12;
    var lo = BLowShelf.ar(sig, frequency.lag(lag), 1, tiltDb);
    var result = BHiShelf.ar(lo, frequency.lag(lag), 1, tiltDb.neg);
    Out.ar(out, result * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_freq_shifter_mono, { |in=1024, out=1026, shift_hz=0, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var wet = FreqShift.ar(sig, shift_hz.lag(lag));
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_limiter_mono, { |in=1024, out=1026, threshold=0.9, release=0.01, ceiling=1.0, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var limited = Limiter.ar(sig, threshold.lag(lag), release.lag(lag));
    Out.ar(out, limited * ceiling.lag(lag) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_pitch_shifter_mono, { |in=1024, out=1026, shift=0, window=0.2, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var ratio = 2 ** (shift.lag(lag) / 12);
    var wet = PitchShift.ar(sig, window.lag(lag), ratio);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_vinyl_mono, { |in=1024, out=1026, wow=0.3, flutter=0.3, noise=0.1, hiss=0.05, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var wowMod = SinOsc.kr(0.3 + (wow * 0.5)) * wow * 0.003;
    var flutterMod = SinOsc.kr(6 + (flutter * 4)) * flutter * 0.001;
    var totalMod = wowMod + flutterMod;
    var modulated = DelayL.ar(sig, 0.05, (0.005 + totalMod).clip(0.0001, 0.05));
    var crackle = Dust.ar(3 + (noise * 20)) * noise * 0.15;
    var hissNoise = HPF.ar(WhiteNoise.ar(hiss * 0.05), 5000);
    var wet = modulated + crackle + hissNoise;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

SynthDef(\imbolc_cabinet_mono, { |in=1024, out=1026, type=0, tone=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var toneFreq = tone.lag(lag);
    var small = LPF.ar(sig.tanh, toneFreq.linexp(0, 1, 800, 5000));
    var large = LPF.ar(sig.tanh, toneFreq.linexp(0, 1, 600, 4000));
    var stack = LPF.ar(BPeakEQ.ar(sig.tanh, 2000, 1, 3), toneFreq.linexp(0, 1, 500, 3500));
    var open = LPF.ar(sig.tanh, toneFreq.linexp(0, 1, 1000, 6000));
    var wet = Select.ar(type, [small, large, stack, open]);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Note: granular effects don't have mono variants since they're inherently converting to stereo output
// Note: stereo_widener doesn't have a mono variant since it's inherently a stereo effect
)
