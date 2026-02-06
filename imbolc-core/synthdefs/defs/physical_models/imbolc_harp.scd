// imbolc_harp SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_harp, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, pedal=0.0, damping=0.1,
    attack=0.001, decay=5.0, sustain=0.0, release=0.5,
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
    var finalDecay = (decay * (1 + decayMod) * (1 - damping.lag(lag) * 0.9)).max(0.01);
    var trig = Trig1.kr(gateSig, 0.001);
    var coef = 0.1.neg;
    var delayTime = finalFreq.reciprocal;
    var exciter = HPF.ar(WhiteNoise.ar, 1000) * EnvGen.ar(Env.perc(0.0001, 0.003), trig);
    var pluck = Pluck.ar(exciter, trig, 0.05, delayTime, finalDecay, coef);
    var sympathetic = CombL.ar(pluck * 0.1, 0.1, finalFreq.reciprocal * 2, 1);
    var sig = pluck + sympathetic;
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
