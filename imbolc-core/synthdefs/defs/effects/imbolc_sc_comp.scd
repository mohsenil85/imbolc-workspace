// imbolc_sc_comp SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_sc_comp, { |in=1024, out=1026, sidechain_in=0, threshold=0.3, ratio=4, attack=0.01, release=0.1, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var scSig = Select.ar(sidechain_in > 0, [sig, In.ar(sidechain_in, 2)]);
    var compressed = Compander.ar(sig, scSig,
        thresh: threshold,
        slopeBelow: 1.0,
        slopeAbove: ratio.reciprocal,
        clampTime: attack,
        relaxTime: release
    );
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (compressed * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
