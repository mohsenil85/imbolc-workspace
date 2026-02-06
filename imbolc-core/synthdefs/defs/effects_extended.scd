// Extended Effects - Convolution Reverb, Vocoder, Ring Mod, Autopan, Resonator,
// Multiband Comp, Para EQ, Spectral Freeze, Glitch, Leslie, Spring Reverb,
// Env Follower, Mid/Side, Crossfader
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

// Convolution Reverb - Uses loaded impulse response buffer
// ir_buffer: SC buffer number of loaded IR (-1 = bypass/passthrough)
SynthDef(\imbolc_convolution_reverb, { |in=1024, out=1026, ir_buffer=(-1), mix=0.3, predelay=0.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var delayed = DelayN.ar(mono, 0.5, predelay.lag(lag));
    // Convolution2 uses partitioned convolution (FFT size 2048)
    var conv = Convolution2.ar(delayed, ir_buffer, 0, 2048);
    var wet = [conv, conv]; // mono IR -> stereo
    // If ir_buffer < 0, pass through dry signal
    var hasIR = (ir_buffer >= 0);
    var output = Select.ar(hasIR, [sig, (sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))]);
    Out.ar(out, output * fadein);
}).writeDefFile(dir);

// Vocoder — analysis bands on input, resynthesis with oscillator bank
SynthDef(\imbolc_vocoder, { |in=1024, out=1026, bands=16, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var wet = Mix.fill(16, { |i|
        var active = (i < bands);
        var freq = (100 * (2 ** (i * 0.5))).min(16000);
        var analysis = Amplitude.kr(BPF.ar(mono, freq, 0.1), 0.01, 0.05);
        SinOsc.ar(freq) * analysis * active;
    });
    wet = [wet, wet];
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Ring Modulator
SynthDef(\imbolc_ringmod, { |in=1024, out=1026, freq=440, depth=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mod = SinOsc.ar(freq.lag(lag));
    var wet = sig * ((1 - depth.lag(lag)) + (mod * depth.lag(lag)));
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Autopan — LFO-driven stereo panning (shape: 0=sine, 1=square, 2=tri)
SynthDef(\imbolc_autopan, { |in=1024, out=1026, rate=2, depth=0.5, shape=0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var lfo = Select.kr(shape, [SinOsc.kr(rate), LFPulse.kr(rate).range(-1,1), LFTri.kr(rate)]) * depth.lag(lag);
    var wet = Pan2.ar(mono, lfo);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Resonator — tuned CombC feedback
SynthDef(\imbolc_resonator, { |in=1024, out=1026, freq=440, decay=1.0, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var delay = freq.lag(lag).reciprocal.clip(0.00005, 0.05);
    var wet = CombC.ar(sig, 0.05, delay, decay.lag(lag));
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Multiband Compressor — 3-band split + independent Compander
SynthDef(\imbolc_multiband_comp, { |in=1024, out=1026, low_thresh=0.3, mid_thresh=0.3, hi_thresh=0.3, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var lo = LPF.ar(sig, 250);
    var mid = BPF.ar(sig, 2000, 1);
    var hi = HPF.ar(sig, 6000);
    var wet = Compander.ar(lo, lo, low_thresh.lag(lag), 1, 0.25, 0.01, 0.1)
            + Compander.ar(mid, mid, mid_thresh.lag(lag), 1, 0.25, 0.01, 0.1)
            + Compander.ar(hi, hi, hi_thresh.lag(lag), 1, 0.25, 0.01, 0.1);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Parametric EQ — 3-band BPeakEQ
SynthDef(\imbolc_para_eq, { |in=1024, out=1026,
    lo_freq=200, lo_gain=0, mid_freq=1000, mid_gain=0, hi_freq=5000, hi_gain=0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    sig = BPeakEQ.ar(sig, lo_freq.lag(lag).clip(20, 20000), 1, lo_gain.lag(lag));
    sig = BPeakEQ.ar(sig, mid_freq.lag(lag).clip(20, 20000), 1, mid_gain.lag(lag));
    sig = BPeakEQ.ar(sig, hi_freq.lag(lag).clip(20, 20000), 1, hi_gain.lag(lag));
    Out.ar(out, sig * fadein);
}).writeDefFile(dir);

// Spectral Freeze — FFT freeze + blur
SynthDef(\imbolc_spectral_freeze, { |in=1024, out=1026, freeze=0, blur=0, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var chain = FFT(LocalBuf(2048), mono);
    var wet;
    chain = PV_MagFreeze(chain, freeze.lag(lag) > 0.5);
    chain = PV_MagSmear(chain, (blur.lag(lag) * 20).asInteger);
    wet = IFFT(chain) ! 2;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Glitch — buffer capture with triggered stutter replay
SynthDef(\imbolc_glitch, { |in=1024, out=1026, rate=4, size=0.1, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var buf = LocalBuf(44100, 1);
    var phase = Phasor.ar(0, 1, 0, 44100);
    var trig = Impulse.ar(rate);
    var readPhase = Latch.ar(phase, trig);
    var wet = BufRd.ar(1, buf,
        Phasor.ar(trig, 1, readPhase, readPhase + (size.lag(lag) * SampleRate.ir)), loop: 1) ! 2;
    BufWr.ar(mono, buf, phase);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Leslie — rotary speaker (horn doppler + drum)
SynthDef(\imbolc_leslie, { |in=1024, out=1026, speed=1, horn_depth=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var hornRate = speed.lag(lag).linlin(0, 2, 0.5, 8);
    var hornMod = SinOsc.kr(hornRate) * horn_depth.lag(lag);
    var horn = DelayL.ar(mono, 0.01, (0.003 + (hornMod * 0.002)).clip(0.0001, 0.01));
    var hornAM = horn * (1 + (SinOsc.kr(hornRate, 0.5pi) * horn_depth.lag(lag) * 0.3));
    var drumRate = hornRate * 0.6;
    var drumMod = SinOsc.kr(drumRate);
    var drum = DelayL.ar(mono, 0.01, (0.004 + (drumMod * 0.001)).clip(0.0001, 0.01));
    var wet = [(hornAM * 0.7) + (drum * 0.3), (DelayL.ar(hornAM, 0.001, 0.0003) * 0.7) + (drum * 0.3)];
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Spring Reverb — allpass diffusion + comb bank
SynthDef(\imbolc_spring_reverb, { |in=1024, out=1026, decay=2.0, tone=0.5, mix=0.3, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var input = AllpassN.ar(AllpassN.ar(mono, 0.05, 0.017, 0.1), 0.05, 0.023, 0.1);
    var wet = Mix([
        CombL.ar(input, 0.2, 0.0397, decay.lag(lag)),
        CombL.ar(input, 0.2, 0.0531, decay.lag(lag) * 0.9),
        CombL.ar(input, 0.2, 0.0673, decay.lag(lag) * 0.8),
        CombL.ar(input, 0.2, 0.0789, decay.lag(lag) * 0.7),
    ]) * 0.25;
    wet = LPF.ar(wet, tone.lag(lag).linexp(0, 1, 500, 8000));
    wet = [wet, DelayL.ar(wet, 0.01, 0.003)];
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Envelope Follower — amplitude-shaping
SynthDef(\imbolc_env_follower, { |in=1024, out=1026, attack=0.01, release=0.1, depth=0.5, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var follower = Amplitude.kr((sig[0] + sig[1]) * 0.5, attack.lag(lag), release.lag(lag));
    var shaped = sig * follower.linlin(0, 1, 1 - depth.lag(lag), 1);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (shaped * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Mid/Side — width + independent mid/side gain
SynthDef(\imbolc_midside, { |in=1024, out=1026, width=1.0, mid_gain=0, side_gain=0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mid = (sig[0] + sig[1]) * 0.5 * (mid_gain.lag(lag) / 6).dbamp;
    var side = (sig[0] - sig[1]) * 0.5 * (side_gain.lag(lag) / 6).dbamp * width.lag(lag);
    Out.ar(out, [mid + side, mid - side] * fadein);
}).writeDefFile(dir);

// Crossfader — A/B crossfade between input and secondary bus
SynthDef(\imbolc_crossfader, { |in=1024, out=1026, crossfade=0.5, bus_b=0, lag=0.02|
    var sigA = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var sigB = Select.ar(bus_b > 0, [DC.ar(0) ! 2, In.ar(bus_b, 2)]);
    var cf = crossfade.lag(lag);
    Out.ar(out, ((sigA * (1 - cf)) + (sigB * cf)) * fadein);
}).writeDefFile(dir);

// Denoise — FFT spectral gate + highpass for background noise suppression
SynthDef(\imbolc_denoise, { |in=1024, out=1026, threshold=0.3, hp_freq=80, smoothing=0.5, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var filtered = HPF.ar(sig, hp_freq.lag(lag).clip(20, 500));
    var chainL = FFT(LocalBuf(2048), filtered[0]);
    var chainR = FFT(LocalBuf(2048), filtered[1]);
    var magThresh = threshold.lag(lag).linlin(0, 1, 0, 10);
    var smearBins = (smoothing.lag(lag) * 10).asInteger;
    var wet;
    chainL = PV_MagAbove(chainL, magThresh);
    chainR = PV_MagAbove(chainR, magThresh);
    chainL = PV_MagSmear(chainL, smearBins);
    chainR = PV_MagSmear(chainR, smearBins);
    wet = [IFFT(chainL), IFFT(chainR)];
    wet = Limiter.ar(wet, 0.95, 0.01);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Autotune — pitch detection + correction to nearest semitone
// speed: 0=slow/natural correction, 1=instant/robotic
// shift: additional pitch shift in semitones
SynthDef(\imbolc_autotune, { |in=1024, out=1026, speed=0.5, shift=0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var freq, hasFreq, midi, targetMidi, correction, ratio, corrected, wet;

    // Detect pitch
    #freq, hasFreq = Pitch.kr(mono, ampThreshold: 0.02, median: 7);

    // Convert to MIDI, quantize to nearest semitone, add shift
    midi = freq.cpsmidi;
    targetMidi = midi.round + shift.lag(lag);

    // Interpolate between detected and target based on speed
    correction = (targetMidi - midi) * speed.lag(lag).linlin(0, 1, 0.1, 1.0);
    ratio = (midi + correction - midi).midiratio;

    // Apply pitch shift (only when pitch detected)
    corrected = PitchShift.ar(mono, 0.2, ratio * hasFreq + (1 - hasFreq), 0, 0.004);
    wet = corrected ! 2;

    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);

// Wah Pedal — resonant bandpass filter with manual or auto-wah mode
// mode: 0=manual (position controls freq), 1=auto (envelope follower)
// position: manual pedal position (0-1)
// resonance: filter Q (0.5-10)
// sensitivity: envelope follower sensitivity for auto mode
SynthDef(\imbolc_wah_pedal, { |in=1024, out=1026, position=0.5, resonance=3, mode=0, sensitivity=0.5, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var env, freqCtrl, freq, rq, wet;

    // Envelope follower for auto-wah
    env = Amplitude.kr(mono, 0.01, 0.1) * sensitivity.lag(lag) * 4;

    // Choose control source: manual position or envelope
    freqCtrl = Select.kr(mode, [position.lag(lag), env.clip(0, 1)]);

    // Map to classic wah frequency range (300Hz - 2500Hz)
    freq = freqCtrl.linexp(0, 1, 300, 2500);

    // Convert resonance to reciprocal Q (rq)
    rq = resonance.lag(lag).reciprocal;

    // Resonant bandpass filter
    wet = BPF.ar(sig, freq, rq) * resonance.lag(lag).sqrt;  // Compensate gain

    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
