// imbolc_delay_mono SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_delay_mono, { |in=1024, out=1026, time=0.3, feedback=0.5, mix=0.3, lag=0.02, time_mod_in=(-1), feedback_mod_in=(-1)|
    var sig = In.ar(in, 1);
    var fadein = Line.kr(0, 1, 0.01);
    var timeMod = Select.kr(time_mod_in >= 0, [0, In.kr(time_mod_in)]);
    var fbMod = Select.kr(feedback_mod_in >= 0, [0, In.kr(feedback_mod_in)]);
    var finalTime = (time.lag(lag) * (1 + timeMod)).clip(0.001, 2.0);
    var finalFb = (feedback.lag(lag) + fbMod).clip(0, 1);
    var delayed = CombL.ar(sig, 2.0, finalTime, finalFb * 4);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (delayed * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
