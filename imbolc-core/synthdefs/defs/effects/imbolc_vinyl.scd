// imbolc_vinyl SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_vinyl, { |in=1024, out=1026, wow=0.3, flutter=0.3, noise=0.1, hiss=0.05, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    // Wow (slow pitch modulation) and flutter (fast pitch modulation)
    var wowMod = SinOsc.kr(0.3 + (wow * 0.5)) * wow * 0.003;
    var flutterMod = SinOsc.kr(6 + (flutter * 4)) * flutter * 0.001;
    var totalMod = wowMod + flutterMod;
    var modulated = DelayL.ar(sig, 0.05, (0.005 + totalMod).clip(0.0001, 0.05));
    // Crackle noise (Dust)
    var crackle = Dust.ar(3 + (noise * 20)) * noise * 0.15;
    // Hiss (filtered noise)
    var hissNoise = HPF.ar(WhiteNoise.ar(hiss * 0.05), 5000);
    var wet = modulated + crackle + hissNoise;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
