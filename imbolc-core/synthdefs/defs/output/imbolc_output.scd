// imbolc_output SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_output, { |in=1024, out=0, level=0.8, mute=0, pan=0, lag=0.02, pan_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var panMod = Select.kr(pan_mod_in >= 0, [0, In.kr(pan_mod_in)]);
    var finalPan = (pan.lag(lag) + panMod).clip(-1, 1);
    var panned = Balance2.ar(sig[0], sig[1], finalPan);
    var muteGain = (1 - mute).lag(lag).clip(0, 1);
    Out.ar(out, panned * level.lag(lag) * muteGain);
}).writeDefFile(dir);
)
