// imbolc_midside SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_midside, { |in=1024, out=1026, width=1.0, mid_gain=0, side_gain=0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mid = (sig[0] + sig[1]) * 0.5 * (mid_gain.lag(lag) / 6).dbamp;
    var side = (sig[0] - sig[1]) * 0.5 * (side_gain.lag(lag) / 6).dbamp * width.lag(lag);
    Out.ar(out, [mid + side, mid - side] * fadein);
}).writeDefFile(dir);
)
