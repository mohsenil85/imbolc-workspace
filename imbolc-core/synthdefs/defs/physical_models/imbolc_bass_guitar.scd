// imbolc_bass_guitar SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_bass_guitar, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=82.4, amp=0.6, lag=0.02, finger=0.5, fret_noise=0.2,
    attack=0.001, decay=2.0, sustain=0.0, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), decay_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01);
    var trig = Trig1.kr(gateSig, 0.001);
    var coef = (finger.lag(lag) * 0.3).neg;
    var delayTime = finalFreq.reciprocal;
    var exciter = LPF.ar(WhiteNoise.ar, 800) * EnvGen.ar(Env.perc(0.0001, 0.008), trig);
    var fretNoise = HPF.ar(WhiteNoise.ar, 2000) * EnvGen.ar(Env.perc(0.0001, 0.01), trig) * fret_noise.lag(lag) * 0.1;
    var pluck = Pluck.ar(exciter, trig, 0.1, delayTime, finalDecay, coef);
    var sig = LPF.ar(pluck, 2000) + fretNoise;
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
