// imbolc_bus_in SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_bus_in, { |out=1024, in=1024, gain=1, instrument_id=0|
    var sig = In.ar(in, 2) * gain;
    SendPeakRMS.kr(sig, 10, 3, "/audio_in_level", instrument_id);
    Out.ar(out, sig);
}).writeDefFile(dir);
)
