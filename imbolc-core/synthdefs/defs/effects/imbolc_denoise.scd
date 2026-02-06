// imbolc_denoise SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_denoise, { |in=1024, out=1026, threshold=0.3, hp_freq=80, smoothing=0.5, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var filtered = HPF.ar(sig, hp_freq.lag(lag).clip(20, 500));
    var chainL = FFT(LocalBuf(2048), filtered[0]);
    var chainR = FFT(LocalBuf(2048), filtered[1]);
    var magThresh = threshold.lag(lag).linlin(0, 1, 0, 10);
    var smearBins = (smoothing.lag(lag) * 10).asInteger;
    var wet;
    chainL = PV_MagAbove(chainL, magThresh);
    chainR = PV_MagAbove(chainR, magThresh);
    chainL = PV_MagSmear(chainL, smearBins);
    chainR = PV_MagSmear(chainR, smearBins);
    wet = [IFFT(chainL), IFFT(chainR)];
    wet = Limiter.ar(wet, 0.95, 0.01);
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
