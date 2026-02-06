// imbolc_granular_freeze SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_granular_freeze, { |in=1024, out=1026, grain_size=0.1, density=10, pitch=0, spread=0.5, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var bufFrames = 44100 * 2;
    var buf = LocalBuf(bufFrames, 1);
    var phase = Phasor.ar(0, 1, 0, bufFrames);
    var pitchRatio, trig, scatter, grainPos, wet;
    BufWr.ar(mono, buf, phase);
    pitchRatio = 2 ** (pitch.lag(lag) / 12);
    trig = Impulse.ar(density);
    // Scatter grain position around current write position
    scatter = TRand.ar(0, spread.lag(lag) * bufFrames, trig);
    grainPos = (phase - scatter).wrap(0, bufFrames) / SampleRate.ir;
    wet = TGrains.ar(2, trig, buf, pitchRatio, grainPos, grain_size.lag(lag), 0, 0.5);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
