// imbolc_tom SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_tom, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=100, amp=0.7, lag=0.02, pitch=100, decay=0.3,
    attack=0.001, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), decay_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [pitch.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01);

    var trig = Trig1.kr(gateSig, 0.001);
    var pitchEnv = EnvGen.kr(Env.perc(0.001, 0.08), trig) * 0.5;
    var ampEnv = EnvGen.kr(Env.perc(attack, finalDecay), trig);
    var sig = SinOsc.ar(finalFreq * (1 + pitchEnv)) * ampEnv;
    sig = sig + (HPF.ar(WhiteNoise.ar, 2000) * EnvGen.kr(Env.perc(0.001, 0.02), trig) * 0.2);
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
