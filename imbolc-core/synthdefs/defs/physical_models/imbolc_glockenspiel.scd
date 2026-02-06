// imbolc_glockenspiel SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_glockenspiel, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=1000, amp=0.4, lag=0.02, hardness=0.5, decay=2.0,
    attack=0.001, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), decay_mod_in=(-1), hardness_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var hardnessMod = Select.kr(hardness_mod_in >= 0, [0, In.kr(hardness_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01);
    var finalHardness = (hardness + hardnessMod).clip(0, 1);
    var trig = Trig1.kr(gateSig, 0.001);
    var modeRatios = [1, 2.76, 5.4, 8.93];
    var modeAmps = [1, 0.5, 0.25, 0.12];
    var exciteWidth = (1 - finalHardness) * 0.005 + 0.0001;
    var exciter = WhiteNoise.ar * EnvGen.ar(Env.perc(0.00001, exciteWidth), trig);
    var sig = HPF.ar(Mix.fill(4, { |i|
        var modeFreq = finalFreq * modeRatios[i];
        var ringTime = finalDecay / (1 + (i * 0.3));
        Ringz.ar(exciter, modeFreq.clip(20, 18000), ringTime) * modeAmps[i];
    }) * 0.2, 400);
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
