// imbolc_bus_out SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_bus_out, { |in=1024, out=0, level=0.8, mute=0, pan=0, lag=0.02|
    var sig = In.ar(in, 2);
    var panned = Balance2.ar(sig[0], sig[1], pan.lag(lag));
    Out.ar(out, panned * level.lag(lag) * (1 - mute));
}).writeDefFile(dir);
)
