// imbolc_lufs_meter SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_lufs_meter, {
    var sig = In.ar(0, 2);
    // Approximate K-weighting: high-shelf boost + high-pass
    var weighted = HPF.ar(BHiShelf.ar(sig, 1500, 1, 4), 38);
    SendPeakRMS.kr(weighted, 20, 3, "/lufs");
}).writeDefFile(dir);
)
