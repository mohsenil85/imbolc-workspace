// imbolc_koto SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_koto, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, bend=0.3, damping=0.3,
    attack=0.001, decay=2.5, sustain=0.0, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), decay_mod_in=(-1), bend_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var bendMod = Select.kr(bend_mod_in >= 0, [0, In.kr(bend_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01);
    var finalBend = (bend + bendMod).clip(0, 1);
    var trig = Trig1.kr(gateSig, 0.001);
    var bendEnv = EnvGen.kr(Env.perc(0.001, 0.15), trig);
    var bendAmount = bendEnv * finalBend * 0.05;
    var bendFreq = finalFreq * (1 - bendAmount);
    var coef = (damping.lag(lag) * 0.3).neg;
    var delayTime = bendFreq.reciprocal;
    var exciter = HPF.ar(WhiteNoise.ar, 800) * EnvGen.ar(Env.perc(0.0001, 0.004), trig);
    var pluck = Pluck.ar(exciter, trig, 0.05, delayTime, finalDecay, coef);
    var sig = pluck + (BPF.ar(pluck, finalFreq * 3, 0.1) * 0.2);
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
