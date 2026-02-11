// imbolc_sampler_oneshot SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_sampler_oneshot, { |out=1024, bufnum=0, sliceStart=0, sliceEnd=1,
                                      rate=1.0, amp=0.8, gate=1|
    var numFrames = BufFrames.kr(bufnum);
    var startFrame = sliceStart * numFrames;
    var endFrame = sliceEnd * numFrames;

    // Calculate duration to drive Line and Envelope
    var safeRate = rate.abs.max(0.001);
    var bufRate = BufSampleRate.kr(bufnum) * safeRate;
    var duration = ((endFrame - startFrame).abs / bufRate).max(0.001);

    // Line.ar drives the pointer from start to end over duration
    var isReverse = rate < 0;
    var lineStart = Select.kr(isReverse, [startFrame, endFrame]);
    var lineEnd = Select.kr(isReverse, [endFrame, startFrame]);
    var phasor = Line.ar(lineStart, lineEnd, duration);

    var sig = LeakDC.ar(BufRd.ar(2, bufnum, phasor, loop: 0, interpolation: 4));

    // Edge envelope removes hard starts/stops at arbitrary slice points.
    var edge = (duration * 0.25).clip(0.001, 0.01);
    edge = edge.min(duration * 0.5);
    var sustain = (duration - (edge * 2)).max(0);
    var env = EnvGen.kr(Env.linen(edge, sustain, edge), doneAction: 2);

    Out.ar(out, sig * amp.lag(0.005) * env);
}).writeDefFile(dir);
)
