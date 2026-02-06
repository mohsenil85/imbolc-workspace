// imbolc_leslie SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_leslie, { |in=1024, out=1026, speed=1, horn_depth=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var hornRate = speed.lag(lag).linlin(0, 2, 0.5, 8);
    var hornMod = SinOsc.kr(hornRate) * horn_depth.lag(lag);
    var horn = DelayL.ar(mono, 0.01, (0.003 + (hornMod * 0.002)).clip(0.0001, 0.01));
    var hornAM = horn * (1 + (SinOsc.kr(hornRate, 0.5pi) * horn_depth.lag(lag) * 0.3));
    var drumRate = hornRate * 0.6;
    var drumMod = SinOsc.kr(drumRate);
    var drum = DelayL.ar(mono, 0.01, (0.004 + (drumMod * 0.001)).clip(0.0001, 0.01));
    var wet = [(hornAM * 0.7) + (drum * 0.3), (DelayL.ar(hornAM, 0.001, 0.0003) * 0.7) + (drum * 0.3)];
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
