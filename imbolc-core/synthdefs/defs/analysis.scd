// Analysis - Metering, spectrum, LUFS, scope, disk recording
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

// Meter - Reads hardware output, sends peak/RMS via /reply
SynthDef(\imbolc_meter, {
    SendPeakRMS.kr(In.ar(0, 2), 10, 3, "/meter");
}).writeDefFile(dir);

// Spectrum Analyzer - 7-band frequency analysis via bandpass filters
// Sends amplitude data at ~20Hz via SendReply
SynthDef(\imbolc_spectrum, {
    var sig = In.ar(0, 2);
    var mono = (sig[0] + sig[1]) * 0.5;
    var freqs = #[60, 150, 400, 1000, 2500, 6000, 15000];
    var bws = #[0.8, 0.7, 0.6, 0.5, 0.5, 0.5, 0.5]; // wider bandwidth at low freqs
    var amps = Array.fill(7, { |i|
        Amplitude.kr(BPF.ar(mono, freqs[i], bws[i]), 0.05, 0.2);
    });
    SendReply.kr(Impulse.kr(20), '/spectrum', amps);
}).writeDefFile(dir);

// LUFS Meter - K-weighted loudness measurement
// Sends peak and RMS at ~20Hz via SendPeakRMS
SynthDef(\imbolc_lufs_meter, {
    var sig = In.ar(0, 2);
    // Approximate K-weighting: high-shelf boost + high-pass
    var weighted = HPF.ar(BHiShelf.ar(sig, 1500, 1, 4), 38);
    SendPeakRMS.kr(weighted, 20, 3, "/lufs");
}).writeDefFile(dir);

// Scope - Sends peak amplitude at ~30Hz for oscilloscope display
SynthDef(\imbolc_scope, {
    var sig = In.ar(0, 2);
    var mono = (sig[0] + sig[1]) * 0.5;
    // Bipolar peak: track both positive and negative peaks
    var peak = Amplitude.kr(mono, 0.001, 0.03);
    // Capture sign by checking current sample polarity
    var sign = Latch.kr(mono, Impulse.kr(30));
    SendReply.kr(Impulse.kr(30), '/scope', [sign.sign * peak]);
}).writeDefFile(dir);

// Disk Recorder - Writes stereo audio from a bus to a disk-backed buffer
SynthDef(\imbolc_disk_record, { |bufnum=0, in=0|
    DiskOut.ar(bufnum, In.ar(in, 2));
}).writeDefFile(dir);
)
