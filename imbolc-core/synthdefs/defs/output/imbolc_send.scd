// imbolc_send SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_send, { |in=1024, out=1026, level=0.0, lag=0.02, level_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var levelMod = Select.kr(level_mod_in >= 0, [0, In.kr(level_mod_in)]);
    var finalLevel = (level.lag(lag) + levelMod).clip(0, 1);
    Out.ar(out, sig * finalLevel);
}).writeDefFile(dir);
)
