// imbolc_spring_reverb SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_spring_reverb, { |in=1024, out=1026, decay=2.0, tone=0.5, mix=0.3, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var input = AllpassN.ar(AllpassN.ar(mono, 0.05, 0.017, 0.1), 0.05, 0.023, 0.1);
    var wet = Mix([
        CombL.ar(input, 0.2, 0.0397, decay.lag(lag)),
        CombL.ar(input, 0.2, 0.0531, decay.lag(lag) * 0.9),
        CombL.ar(input, 0.2, 0.0673, decay.lag(lag) * 0.8),
        CombL.ar(input, 0.2, 0.0789, decay.lag(lag) * 0.7),
    ]) * 0.25;
    wet = LPF.ar(wet, tone.lag(lag).linexp(0, 1, 500, 8000));
    wet = [wet, DelayL.ar(wet, 0.01, 0.003)];
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
