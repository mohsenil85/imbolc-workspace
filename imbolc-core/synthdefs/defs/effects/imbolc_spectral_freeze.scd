// imbolc_spectral_freeze SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_spectral_freeze, { |in=1024, out=1026, freeze=0, blur=0, mix=0.5, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var chain = FFT(LocalBuf(2048), mono);
    var wet;
    chain = PV_MagFreeze(chain, freeze.lag(lag) > 0.5);
    chain = PV_MagSmear(chain, (blur.lag(lag) * 20).asInteger);
    wet = IFFT(chain) ! 2;
    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
