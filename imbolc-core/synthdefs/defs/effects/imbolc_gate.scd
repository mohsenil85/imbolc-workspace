// imbolc_gate SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_gate, { |in=1024, out=1026, rate=4, depth=1, shape=1, lag=0.02, rate_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var rateMod = Select.kr(rate_mod_in >= 0, [0, In.kr(rate_mod_in)]);
    var finalRate = (rate.lag(lag) * (1 + rateMod)).max(0.1);
    var depthLag = depth.lag(lag);
    var sine = SinOsc.kr(finalRate).range(1 - depthLag, 1);
    var square = LFPulse.kr(finalRate, width: 0.5).range(1 - depthLag, 1);
    var saw = LFSaw.kr(finalRate).range(1 - depthLag, 1);
    var lfo = Select.kr(shape, [sine, square, saw]);
    Out.ar(out, sig * lfo * fadein);
}).writeDefFile(dir);
)
