// imbolc_autotune SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_autotune, { |in=1024, out=1026, speed=0.5, shift=0, mix=1.0, lag=0.02|
    var sig = In.ar(in, 2);
    var fadein = Line.kr(0, 1, 0.01);
    var mono = (sig[0] + sig[1]) * 0.5;
    var freq, hasFreq, midi, targetMidi, correction, ratio, corrected, wet;

    // Detect pitch
    #freq, hasFreq = Pitch.kr(mono, ampThreshold: 0.02, median: 7);

    // Convert to MIDI, quantize to nearest semitone, add shift
    midi = freq.cpsmidi;
    targetMidi = midi.round + shift.lag(lag);

    // Interpolate between detected and target based on speed
    correction = (targetMidi - midi) * speed.lag(lag).linlin(0, 1, 0.1, 1.0);
    ratio = (midi + correction - midi).midiratio;

    // Apply pitch shift (only when pitch detected)
    corrected = PitchShift.ar(mono, 0.2, ratio * hasFreq + (1 - hasFreq), 0, 0.004);
    wet = corrected ! 2;

    Out.ar(out, ((sig * (1 - mix.lag(lag))) + (wet * mix.lag(lag))) * fadein);
}).writeDefFile(dir);
)
