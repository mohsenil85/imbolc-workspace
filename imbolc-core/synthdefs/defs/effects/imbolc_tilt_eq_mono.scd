// imbolc_tilt_eq_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_tilt_eq_mono, { |in=1024, out=1026, tilt=0, frequency=1000, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var tiltDb = tilt.lag(lag) * 12;
    var lo = BLowShelf.ar(sig, frequency.lag(lag), 1, tiltDb);
    var result = BHiShelf.ar(lo, frequency.lag(lag), 1, tiltDb.neg);
    Out.ar(out, result * fadein);
}).writeDefFile(dir);
)
