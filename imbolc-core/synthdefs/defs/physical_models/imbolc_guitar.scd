// imbolc_guitar SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_guitar, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, pick_pos=0.1, damping=0.5, body=0.5,
    attack=0.001, decay=3.0, sustain=0.0, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), decay_mod_in=(-1), damping_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var dampingMod = Select.kr(damping_mod_in >= 0, [0, In.kr(damping_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01);
    var finalDamping = (damping + dampingMod).clip(0, 1);
    var trig = Trig1.kr(gateSig, 0.001);
    var coef = (finalDamping * 0.5).neg;
    var delayTime = finalFreq.reciprocal;
    var exciter = BPF.ar(WhiteNoise.ar, finalFreq * (3 + (pick_pos.lag(lag) * 5)), 0.5) * EnvGen.ar(Env.perc(0.0001, 0.005), trig);
    var pluck = Pluck.ar(exciter, trig, 0.05, delayTime, finalDecay, coef);
    var bodyRes = BPF.ar(pluck, 100, 0.3) + BPF.ar(pluck, 220, 0.2);
    var sig = pluck + (bodyRes * body.lag(lag) * 0.5);
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
