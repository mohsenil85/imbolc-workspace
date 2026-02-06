// imbolc_clave SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_clave, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=2500, amp=0.5, lag=0.02, tone=2500, decay=0.03,
    attack=0.001, release=0.03,
    amp_mod_in=(-1), pitch_mod_in=(-1), decay_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [tone.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var finalTone = freqSig * (2 ** pitchMod);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);

    var trig = Trig1.kr(gateSig, 0.001);
    var env = EnvGen.kr(Env.perc(attack, finalDecay, curve: -8), trig);
    var sig = SinOsc.ar(finalTone) + SinOsc.ar(finalTone * 1.5, 0, 0.3);
    sig = sig * env;
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
