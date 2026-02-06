// imbolc_stereo_widener SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_stereo_widener, { |in=1024, out=1026, width=1, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mid = (sig[0] + sig[1]) * 0.5;
    var side = (sig[0] - sig[1]) * 0.5;
    var widthLag = width.lag(lag);
    var wet = [mid + (side * widthLag), mid - (side * widthLag)];
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
