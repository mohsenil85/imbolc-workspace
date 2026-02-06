// imbolc_lfo SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_lfo, { |out=0, rate=1, depth=0.5, shape=0|
    var sine = SinOsc.kr(rate);
    var square = LFPulse.kr(rate).range(-1, 1);
    var saw = LFSaw.kr(rate);
    var tri = LFTri.kr(rate);
    var sig = Select.kr(shape, [sine, square, saw, tri]) * depth;
    Out.kr(out, sig);
}).writeDefFile(dir);
)
