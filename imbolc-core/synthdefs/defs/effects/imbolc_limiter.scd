// imbolc_limiter SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_limiter, { |in=1024, out=1026, threshold=0.9, release=0.01, ceiling=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var limited = Limiter.ar(sig, threshold.lag(lag), release.lag(lag));
    Out.ar(out, limited * ceiling.lag(lag) * fadein);
}).writeDefFile(dir);
)
