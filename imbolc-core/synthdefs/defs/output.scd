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

// Output Mono - Reads mono signal, pans to stereo using Pan2
SynthDef(\imbolc_output_mono, { |in=1024, level=0.8, mute=0, pan=0, lag=0.02, pan_mod_in=(-1)|
    var sig = In.ar(in, 1);
    var panMod = Select.kr(pan_mod_in >= 0, [0, In.kr(pan_mod_in)]);
    var finalPan = (pan.lag(lag) + panMod).clip(-1, 1);
    var panned = Pan2.ar(sig, finalPan);
    Out.ar(0, panned * level.lag(lag) * (1 - mute));
}).writeDefFile(dir);

// Send - Reads from source bus, writes to a bus's audio bus at send level
SynthDef(\imbolc_send, { |in=1024, out=1026, level=0.0, lag=0.02, level_mod_in=(-1)|
    var sig = In.ar(in, 2);
    var levelMod = Select.kr(level_mod_in >= 0, [0, In.kr(level_mod_in)]);
    var finalLevel = (level.lag(lag) + levelMod).clip(0, 1);
    Out.ar(out, sig * finalLevel);
}).writeDefFile(dir);

// Send Mono - Mono version for mono instruments
SynthDef(\imbolc_send_mono, { |in=1024, out=1026, level=0.0, lag=0.02, level_mod_in=(-1)|
    var sig = In.ar(in, 1);
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

// Click track / metronome - simple sine click with fast envelope
// Outputs directly to hardware (stereo)
SynthDef(\imbolc_click, { |freq=1000, amp=0.5|
    var sig = SinOsc.ar(freq) * EnvGen.kr(Env.perc(0.001, 0.05), doneAction: 2);
    Out.ar(0, (sig * amp).dup);
}).writeDefFile(dir);
)
