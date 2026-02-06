// imbolc_convolution_reverb SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_convolution_reverb, { |in=1024, out=1026, ir_buffer=(-1), mix=0.3, predelay=0.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var delayed = DelayN.ar(mono, 0.5, predelay.lag(lag));
    // Convolution2 uses partitioned convolution (FFT size 2048)
    var conv = Convolution2.ar(delayed, ir_buffer, 0, 2048);
    var wet = [conv, conv]; // mono IR -> stereo
    // If ir_buffer < 0, pass through dry signal
    var hasIR = (ir_buffer >= 0);
    var output = Select.ar(hasIR, [sig, (sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))]);
    Out.ar(out, output * fadein);
}).writeDefFile(dir);
)
