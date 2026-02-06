// imbolc_tilt_eq SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_tilt_eq, { |in=1024, out=1026, tilt=0, frequency=1000, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var tiltDb = tilt.lag(lag) * 12; // -12 to +12 dB
    var lo = BLowShelf.ar(sig, frequency.lag(lag), 1, tiltDb);
    var result = BHiShelf.ar(lo, frequency.lag(lag), 1, tiltDb.neg);
    Out.ar(out, result * fadein);
}).writeDefFile(dir);
)
