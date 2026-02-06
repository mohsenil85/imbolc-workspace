// imbolc_cowbell SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_cowbell, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=560, amp=0.5, lag=0.02, detune=1.5, decay=0.4,
    attack=0.001, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), decay_mod_in=(-1), detune_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01);
    var finalDetune = detune * (1 + detuneMod).max(0.1);

    var trig = Trig1.kr(gateSig, 0.001);
    var env = EnvGen.kr(Env.perc(attack, finalDecay, curve: -6), trig);
    var osc1 = Pulse.ar(finalFreq, 0.5);
    var osc2 = Pulse.ar(finalFreq * finalDetune, 0.5);
    var sig = BPF.ar(osc1 + osc2, 2500, 0.5) * env;
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
