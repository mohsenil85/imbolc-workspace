// imbolc_glitch SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_glitch, { |in=1024, out=1026, rate=4, size=0.1, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var buf = LocalBuf(44100, 1);
    var phase = Phasor.ar(0, 1, 0, 44100);
    var trig = Impulse.ar(rate);
    var readPhase = Latch.ar(phase, trig);
    var wet = BufRd.ar(1, buf,
        Phasor.ar(trig, 1, readPhase, readPhase + (size.lag(lag) * SampleRate.ir)), loop: 1) ! 2;
    BufWr.ar(mono, buf, phase);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
