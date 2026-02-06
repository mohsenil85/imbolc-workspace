// imbolc_sync SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_sync, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1), freq=440, amp=0.5, lag=0.02, sync_ratio=2.0, attack=0.01, decay=0.1, sustain=0.7, release=0.3, amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1), attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1), sync_ratio_mod_in=(-1)|
    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
    var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
    var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
    var attackMod = Select.kr(attack_mod_in >= 0, [0, In.kr(attack_mod_in)]);
    var releaseMod = Select.kr(release_mod_in >= 0, [0, In.kr(release_mod_in)]);
    var decayMod = Select.kr(decay_mod_in >= 0, [0, In.kr(decay_mod_in)]);
    var sustainMod = Select.kr(sustain_mod_in >= 0, [0, In.kr(sustain_mod_in)]);
    var syncRatioMod = Select.kr(sync_ratio_mod_in >= 0, [0, In.kr(sync_ratio_mod_in)]);
    var finalFreq = freqSig * (2 ** pitchMod) * (1 + (detuneMod * 0.01));
    var finalAmp = amp.lag(lag) * (1 + ampMod).max(0);
    var finalAttack = (attack * (1 + attackMod)).max(0.001);
    var finalRelease = (release * (1 + releaseMod)).max(0.001);
    var finalDecay = (decay * (1 + decayMod)).max(0.001);
    var finalSustain = (sustain + sustainMod).clip(0, 1);
    var finalSyncRatio = sync_ratio * (1 + syncRatioMod);
    var sig = SyncSaw.ar(finalFreq, finalFreq * finalSyncRatio) * finalAmp * velSig;
    var env = EnvGen.kr(Env.adsr(finalAttack, finalDecay, finalSustain, finalRelease), gateSig);
    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);
)
