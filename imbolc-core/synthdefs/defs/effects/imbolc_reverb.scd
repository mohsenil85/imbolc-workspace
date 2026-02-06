// imbolc_reverb SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_reverb, { |in=1024, out=1026, room=0.5, damp=0.5, mix=0.3, lag=0.02, mix_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mixMod = Select.kr(mix_mod_in >= 0, [0, In.kr(mix_mod_in)]);
    var finalMix = (mix.lag(lag) + mixMod).clip(0, 1);
    var wet = FreeVerb2.ar(sig[0], sig[1], finalMix, room.lag(lag), damp.lag(lag));
    Out.ar(out, wet * fadein);
}).writeDefFile(dir);
)
