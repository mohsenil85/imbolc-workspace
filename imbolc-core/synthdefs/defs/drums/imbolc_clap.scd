// imbolc_clap SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_clap, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=1000, amp=0.6, lag=0.02, spread=0.02, decay=0.3,
    attack=0.001, release=0.1,
    amp_mod_in=(-1), decay_mod_in=(-1), spread_mod_in=(-1)|

    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var spreadMod = Select.kr(spread_mod_in >= 0, [0, In.kr(spread_mod_in)]);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01);
    var finalSpread = (spread + (spreadMod * 0.02)).clip(0.005, 0.1);

    var trig = Trig1.kr(gateSig, 0.001);
    var burst1 = EnvGen.kr(Env.perc(0.001, 0.01), trig);
    var burst2 = EnvGen.kr(Env.perc(0.001, 0.01), TDelay.kr(trig, finalSpread));
    var burst3 = EnvGen.kr(Env.perc(0.001, 0.01), TDelay.kr(trig, finalSpread * 2));
    var tail = EnvGen.kr(Env.perc(0.001, finalDecay), TDelay.kr(trig, finalSpread * 2.5));
    var sig = BPF.ar(WhiteNoise.ar, 1200, 0.5);
    sig = (sig * burst1) + (sig * burst2 * 0.9) + (sig * burst3 * 0.8) + (sig * tail * 0.7);
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
