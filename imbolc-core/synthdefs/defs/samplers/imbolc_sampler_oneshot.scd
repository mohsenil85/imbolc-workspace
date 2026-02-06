// imbolc_sampler_oneshot SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_sampler_oneshot, { |out=1024, bufnum=0, sliceStart=0, sliceEnd=1,
                                      rate=1.0, amp=0.8, gate=1|
    var numFrames = BufFrames.kr(bufnum);
    var startFrame = sliceStart * numFrames;
    var endFrame = sliceEnd * numFrames;

    // Calculate duration to drive Line and Envelope
    var bufRate = BufSampleRate.kr(bufnum) * rate.abs;
    var duration = (endFrame - startFrame).abs / bufRate;

    // Line.ar drives the pointer from start to end over duration
    var isReverse = rate < 0;
    var lineStart = Select.kr(isReverse, [startFrame, endFrame]);
    var lineEnd = Select.kr(isReverse, [endFrame, startFrame]);
    var phasor = Line.ar(lineStart, lineEnd, duration, doneAction: 2);

    var sig = BufRd.ar(2, bufnum, phasor, loop: 0);

    // Simple envelope to de-click
    var attack = 0.002;
    var release = 0.01;
    // Ensure sustain is not negative if duration is very short
    var sustain = (duration - attack - release).max(0);
    var env = EnvGen.kr(Env.linen(attack, sustain, release), doneAction: 2);

    Out.ar(out, sig * amp * env);
}).writeDefFile(dir);
)
