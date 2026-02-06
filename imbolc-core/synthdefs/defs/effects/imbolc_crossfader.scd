// imbolc_crossfader SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_crossfader, { |in=1024, out=1026, crossfade=0.5, bus_b=0, lag=0.02|
    var sigA = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var sigB = Select.ar(bus_b > 0, [DC.ar(0) ! 2, In.ar(bus_b, 2)]);
    var cf = crossfade.lag(lag);
    Out.ar(out, ((sigA * (1 - cf)) + (sigB * cf)) * fadein);
}).writeDefFile(dir);
)
