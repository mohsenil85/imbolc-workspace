// imbolc_vibes SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_vibes, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, motor_speed=5.0, damper=0.0,
    attack=0.001, decay=3.0, sustain=0.0, release=0.1,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
    decay_mod_in=(-1), sustain_mod_in=(-1), motor_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var motorMod = Select.kr(motor_mod_in >= 0, [0, In.kr(motor_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalDecay = (decay * (1 + decayMod)).max(0.01) * (1 - (damper.lag(lag) * 0.9));
    var finalMotor = (motor_speed + (motorMod * 2)).clip(0, 10);
    var trig = Trig1.kr(gateSig, 0.001);
    var modeRatios = [1, 2.0, 3.0, 4.17, 5.43];
    var modeAmps = [1, 0.5, 0.3, 0.2, 0.1];
    var tremolo = SinOsc.kr(finalMotor).range(0.5, 1);
    var sig = Mix.fill(5, { |i|
        var modeFreq = finalFreq * modeRatios[i];
        var modeDecay = finalDecay * [1, 0.8, 0.6, 0.5, 0.4][i];
        SinOsc.ar(modeFreq) * modeAmps[i] * EnvGen.kr(Env.perc(0.001, modeDecay), trig);
    }) * tremolo;
    Out.ar(out, (sig * finalAmp * velSig) ! 2);
}).writeDefFile(dir);
)
