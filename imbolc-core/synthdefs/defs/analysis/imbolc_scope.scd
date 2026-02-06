// imbolc_scope SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_scope, {
    var sig = In.ar(0, 2);
    var mono = (sig[0] + sig[1]) * 0.5;
    // Bipolar peak: track both positive and negative peaks
    var peak = Amplitude.kr(mono, 0.001, 0.03);
    // Capture sign by checking current sample polarity
    var sign = Latch.kr(mono, Impulse.kr(30));
    SendReply.kr(Impulse.kr(30), '/scope', [sign.sign * peak]);
}).writeDefFile(dir);
)
