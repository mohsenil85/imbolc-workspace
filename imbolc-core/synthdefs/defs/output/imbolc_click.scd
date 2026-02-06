// imbolc_click SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_click, { |freq=1000, amp=0.5|
    var sig = SinOsc.ar(freq) * EnvGen.kr(Env.perc(0.001, 0.05), doneAction: 2);
    Out.ar(0, (sig * amp).dup);
}).writeDefFile(dir);
)
