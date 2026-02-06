// imbolc_meter SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_meter, {
    SendPeakRMS.kr(In.ar(0, 2), 10, 3, "/meter");
}).writeDefFile(dir);
)
