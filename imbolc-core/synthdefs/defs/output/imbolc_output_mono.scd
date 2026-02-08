// imbolc_output_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_output_mono, { |in=1024, out=0, level=0.8, mute=0, pan=0, lag=0.02, pan_mod_in=(-1)|
    var sig = In.ar(in, 1);
    var panMod = Select.kr(pan_mod_in >= 0, [0, In.kr(pan_mod_in)]);
    var finalPan = (pan.lag(lag) + panMod).clip(-1, 1);
    var panned = Pan2.ar(sig, finalPan);
    Out.ar(out, panned * level.lag(lag) * (1 - mute));
}).writeDefFile(dir);
)
