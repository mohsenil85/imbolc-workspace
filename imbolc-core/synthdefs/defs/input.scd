// Audio Input and Bus Input
(
var dir = thisProcess.nowExecutingPath.dirname.dirname;

// Audio Input - Live audio from hardware input
SynthDef(\imbolc_audio_in, { |out=1024, channel=0, gain=1, test_tone=0, test_freq=440, instrument_id=0|
    var input = SoundIn.ar(channel) * gain;
    var test = SinOsc.ar(test_freq) * 0.3 * test_tone;
    var sig = input + test;
    // Send peak amplitude for waveform display (10 times per second)
    SendPeakRMS.kr(sig, 10, 3, "/audio_in_level", instrument_id);
    Out.ar(out, sig ! 2);
}).writeDefFile(dir);

// Bus Input - Read audio from an internal mixer bus
SynthDef(\imbolc_bus_in, { |out=1024, in=1024, gain=1, instrument_id=0|
    var sig = In.ar(in, 2) * gain;
    SendPeakRMS.kr(sig, 10, 3, "/audio_in_level", instrument_id);
    Out.ar(out, sig);
}).writeDefFile(dir);
)
