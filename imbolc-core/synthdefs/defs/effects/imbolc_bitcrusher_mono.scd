// imbolc_bitcrusher_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_bitcrusher_mono, { |in=1024, out=1026, rate=8000, bits=8, mix=0.5, lag=0.02|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var crushed = Latch.ar(sig, Impulse.ar(rate));
    var bitDepth = (2 ** bits);
    crushed = (crushed * bitDepth).round / bitDepth;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (crushed * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
