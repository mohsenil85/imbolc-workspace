// imbolc_snare SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_snare, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=180, amp=0.7, lag=0.02, tone_freq=180, noise_amt=0.7, snap=0.8, decay=0.2,
    attack=0.001, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), decay_mod_in=(-1), noise_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [tone_freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var noiseMod = Select.kr(noise_mod_in >= 0, [0, In.kr(noise_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01);
    var finalNoise = (noise_amt + noiseMod).clip(0, 1);

    var trig = Trig1.kr(gateSig, 0.001);
    var toneEnv = EnvGen.kr(Env.perc(attack, finalDecay * 0.5), trig);
    var noiseEnv = EnvGen.kr(Env.perc(attack, finalDecay), trig);
    var snapEnv = EnvGen.kr(Env.perc(0.001, 0.03), trig);
    var tone = SinOsc.ar(finalFreq) * toneEnv * (1 - finalNoise);
    var noise = HPF.ar(WhiteNoise.ar, 1500) * noiseEnv * finalNoise;
    var snapSig = HPF.ar(WhiteNoise.ar, 2500) * snapEnv * snap.lag(lag);
    var sig = tone + noise + snapSig;
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
