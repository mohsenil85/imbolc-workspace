// imbolc_granular_delay SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_granular_delay, { |in=1024, out=1026, time=0.3, grain_size=0.1, density=10, pitch=0, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var bufFrames = 44100 * 2; // 2 second buffer
    var buf = LocalBuf(bufFrames, 1);
    var phase = Phasor.ar(0, 1, 0, bufFrames);
    var readPos, pitchRatio, trig, wet;
    BufWr.ar(mono, buf, phase);
    readPos = (phase - (time.lag(lag) * SampleRate.ir)).wrap(0, bufFrames);
    pitchRatio = 2 ** (pitch.lag(lag) / 12);
    trig = Impulse.ar(density);
    wet = TGrains.ar(2, trig, buf, pitchRatio, readPos / SampleRate.ir, grain_size.lag(lag), 0, 0.5);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
