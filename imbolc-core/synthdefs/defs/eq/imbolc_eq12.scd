// imbolc_eq12 SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_eq12, { |in=1024, out=1026, lag=0.02,
    b0_freq=40,    b0_gain=0, b0_q=0.7, b0_on=1,
    b1_freq=80,    b1_gain=0, b1_q=1.0, b1_on=1,
    b2_freq=160,   b2_gain=0, b2_q=1.0, b2_on=1,
    b3_freq=320,   b3_gain=0, b3_q=1.0, b3_on=1,
    b4_freq=640,   b4_gain=0, b4_q=1.0, b4_on=1,
    b5_freq=1200,  b5_gain=0, b5_q=1.0, b5_on=1,
    b6_freq=2500,  b6_gain=0, b6_q=1.0, b6_on=1,
    b7_freq=5000,  b7_gain=0, b7_q=1.0, b7_on=1,
    b8_freq=8000,  b8_gain=0, b8_q=1.0, b8_on=1,
    b9_freq=12000, b9_gain=0, b9_q=1.0, b9_on=1,
    b10_freq=16000, b10_gain=0, b10_q=1.0, b10_on=1,
    b11_freq=18000, b11_gain=0, b11_q=0.7, b11_on=1|

    var sig = In.ar(in, 2);
    var b0f, b0g, b0r, b0;
    var b1f, b1g, b1r, b1;
    var b2f, b2g, b2r, b2;
    var b3f, b3g, b3r, b3;
    var b4f, b4g, b4r, b4;
    var b5f, b5g, b5r, b5;
    var b6f, b6g, b6r, b6;
    var b7f, b7g, b7r, b7;
    var b8f, b8g, b8r, b8;
    var b9f, b9g, b9r, b9;
    var b10f, b10g, b10r, b10;
    var b11f, b11g, b11r, b11;
    var fadein = Line.kr(0, 1, 0.01);

    // Band 0: Low Shelf
    b0f = b0_freq.lag(lag).clip(20, 20000);
    b0g = b0_gain.lag(lag);
    b0r = b0_q.lag(lag).reciprocal.clip(0.1, 10);
    b0 = BLowShelf.ar(sig, b0f, b0r, b0g);
    sig = Select.ar(b0_on, [sig, b0]);

    // Band 1: Peaking
    b1f = b1_freq.lag(lag).clip(20, 20000);
    b1g = b1_gain.lag(lag);
    b1r = b1_q.lag(lag).reciprocal.clip(0.1, 10);
    b1 = BPeakEQ.ar(sig, b1f, b1r, b1g);
    sig = Select.ar(b1_on, [sig, b1]);

    // Band 2: Peaking
    b2f = b2_freq.lag(lag).clip(20, 20000);
    b2g = b2_gain.lag(lag);
    b2r = b2_q.lag(lag).reciprocal.clip(0.1, 10);
    b2 = BPeakEQ.ar(sig, b2f, b2r, b2g);
    sig = Select.ar(b2_on, [sig, b2]);

    // Band 3: Peaking
    b3f = b3_freq.lag(lag).clip(20, 20000);
    b3g = b3_gain.lag(lag);
    b3r = b3_q.lag(lag).reciprocal.clip(0.1, 10);
    b3 = BPeakEQ.ar(sig, b3f, b3r, b3g);
    sig = Select.ar(b3_on, [sig, b3]);

    // Band 4: Peaking
    b4f = b4_freq.lag(lag).clip(20, 20000);
    b4g = b4_gain.lag(lag);
    b4r = b4_q.lag(lag).reciprocal.clip(0.1, 10);
    b4 = BPeakEQ.ar(sig, b4f, b4r, b4g);
    sig = Select.ar(b4_on, [sig, b4]);

    // Band 5: Peaking
    b5f = b5_freq.lag(lag).clip(20, 20000);
    b5g = b5_gain.lag(lag);
    b5r = b5_q.lag(lag).reciprocal.clip(0.1, 10);
    b5 = BPeakEQ.ar(sig, b5f, b5r, b5g);
    sig = Select.ar(b5_on, [sig, b5]);

    // Band 6: Peaking
    b6f = b6_freq.lag(lag).clip(20, 20000);
    b6g = b6_gain.lag(lag);
    b6r = b6_q.lag(lag).reciprocal.clip(0.1, 10);
    b6 = BPeakEQ.ar(sig, b6f, b6r, b6g);
    sig = Select.ar(b6_on, [sig, b6]);

    // Band 7: Peaking
    b7f = b7_freq.lag(lag).clip(20, 20000);
    b7g = b7_gain.lag(lag);
    b7r = b7_q.lag(lag).reciprocal.clip(0.1, 10);
    b7 = BPeakEQ.ar(sig, b7f, b7r, b7g);
    sig = Select.ar(b7_on, [sig, b7]);

    // Band 8: Peaking
    b8f = b8_freq.lag(lag).clip(20, 20000);
    b8g = b8_gain.lag(lag);
    b8r = b8_q.lag(lag).reciprocal.clip(0.1, 10);
    b8 = BPeakEQ.ar(sig, b8f, b8r, b8g);
    sig = Select.ar(b8_on, [sig, b8]);

    // Band 9: Peaking
    b9f = b9_freq.lag(lag).clip(20, 20000);
    b9g = b9_gain.lag(lag);
    b9r = b9_q.lag(lag).reciprocal.clip(0.1, 10);
    b9 = BPeakEQ.ar(sig, b9f, b9r, b9g);
    sig = Select.ar(b9_on, [sig, b9]);

    // Band 10: Peaking
    b10f = b10_freq.lag(lag).clip(20, 20000);
    b10g = b10_gain.lag(lag);
    b10r = b10_q.lag(lag).reciprocal.clip(0.1, 10);
    b10 = BPeakEQ.ar(sig, b10f, b10r, b10g);
    sig = Select.ar(b10_on, [sig, b10]);

    // Band 11: High Shelf
    b11f = b11_freq.lag(lag).clip(20, 20000);
    b11g = b11_gain.lag(lag);
    b11r = b11_q.lag(lag).reciprocal.clip(0.1, 10);
    b11 = BHiShelf.ar(sig, b11f, b11r, b11g);
    sig = Select.ar(b11_on, [sig, b11]);

    Out.ar(out, sig * fadein);
}).writeDefFile(dir);
)
