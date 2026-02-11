// imbolc_sampler SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_sampler, { |out=1024, bufnum=0, sliceStart=0, sliceEnd=1,
                             rate=1.0, rate_in=(-1), amp=0.8, lag=0.02, loop=0,
                             freq_in=(-1), gate_in=(-1), vel_in=(-1),
                             attack=0.001, decay=0.1, sustain=1.0, release=0.05,
                             amp_mod_in=(-1), srate_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1),
                            decay_mod_in=(-1), sustain_mod_in=(-1)|
    var rateSig = Select.kr(rate_in >= 0, [rate.lag(lag), In.kr(rate_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var srateMod = Select.kr(srate_mod_in >= 0, [0, In.kr(srate_mod_in)]);
    var attackMod = Select.kr(attack_mod_in >= 0, [0, In.kr(attack_mod_in)]);
    var releaseMod = Select.kr(release_mod_in >= 0, [0, In.kr(release_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var sustainMod = Select.kr(sustain_mod_in >= 0, [0, In.kr(sustain_mod_in)]);
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.005);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    // If freq_in is provided, calculate rate from pitch (relative to middle C = 60)
    var freqSig = Select.kr(freq_in >= 0, [60.midicps, In.kr(freq_in)]);
    var pitchRate = freqSig / 60.midicps;
    var finalRate = rateSig * pitchRate * (1 + srateMod);
    var numFrames = BufFrames.kr(bufnum);
    var startFrame = sliceStart * numFrames;
    var endFrame = sliceEnd * numFrames;
    var isReverse = finalRate < 0;
    var phasorStart = Select.kr(isReverse, [startFrame, endFrame]);
    var phasorEnd = Select.kr(isReverse, [endFrame, startFrame]);
    var phasorReset = Select.kr(isReverse, [startFrame, endFrame]);
    var phasor = Phasor.ar(
        trig: 0,
        rate: BufRateScale.kr(bufnum) * finalRate,
        start: phasorStart,
        end: phasorEnd,
        resetPos: phasorReset
    );
    var sig = LeakDC.ar(BufRd.ar(2, bufnum, phasor, loop: loop, interpolation: 4));
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig, doneAction: 2);
    Out.ar(out, sig * finalAmp * env * velSig);
}).writeDefFile(dir);
)
