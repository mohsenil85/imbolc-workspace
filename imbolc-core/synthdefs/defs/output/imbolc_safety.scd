// imbolc_safety SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_safety, { |ceiling=0.95|
    ReplaceOut.ar(0, Limiter.ar(In.ar(0, 2), ceiling, 0.01));
}).writeDefFile(dir);
)
