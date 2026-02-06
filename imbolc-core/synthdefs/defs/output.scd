// Output - Final stage, reads from audio bus, writes to hardware out
// Includes level, mute, and pan controls for mixer integration
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

SynthDef(\imbolc_output, { |in=1024, level=0.8, mute=0, pan=0, lag=0.02, pan_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var panMod = Select.kr(pan_mod_in >= 0, [0, In.kr(pan_mod_in)]);
    var finalPan = (pan.lag(lag) + panMod).clip(-1, 1);
    var panned = Balance2.ar(sig[0], sig[1], finalPan);
    Out.ar(0, panned * level.lag(lag) * (1 - mute));
}).writeDefFile(dir);

// Send - Reads from source bus, writes to a bus's audio bus at send level
SynthDef(\imbolc_send, { |in=1024, out=1026, level=0.0, lag=0.02, level_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var levelMod = Select.kr(level_mod_in >= 0, [0, In.kr(level_mod_in)]);
    var finalLevel = (level.lag(lag) + levelMod).clip(0, 1);
    Out.ar(out, sig * finalLevel);
}).writeDefFile(dir);

// Bus Output - Reads from bus audio bus, applies level/mute/pan, writes to hw
SynthDef(\imbolc_bus_out, { |in=1024, level=0.8, mute=0, pan=0, lag=0.02|
    var sig = In.ar(in, 2);
    var panned = Balance2.ar(sig[0], sig[1], pan.lag(lag));
    Out.ar(0, panned * level.lag(lag) * (1 - mute));
}).writeDefFile(dir);

SynthDef(\imbolc_safety, { |ceiling=0.95|
    ReplaceOut.ar(0, Limiter.ar(In.ar(0, 2), ceiling, 0.01));
}).writeDefFile(dir);
)
