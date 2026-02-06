// imbolc_reverb_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_reverb_mono, { |in=1024, out=1026, room=0.5, damp=0.5, mix=0.3, lag=0.02, mix_mod_in=(-1)|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var mixMod = Select.kr(mix_mod_in >= 0, [0, In.kr(mix_mod_in)]);
    var finalMix = (mix.lag(lag) + mixMod).clip(0, 1);
    var wet = FreeVerb.ar(sig, finalMix, room.lag(lag), damp.lag(lag));
    Out.ar(out, wet * fadein);
}).writeDefFile(dir);
)
