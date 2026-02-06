// imbolc_env_follower SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_env_follower, { |in=1024, out=1026, attack=0.01, release=0.1, depth=0.5, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var follower = Amplitude.kr((sig[0] + sig[1]) * 0.5, attack.lag(lag), release.lag(lag));
    var shaped = sig * follower.linlin(0, 1, 1 - depth.lag(lag), 1);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (shaped * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
