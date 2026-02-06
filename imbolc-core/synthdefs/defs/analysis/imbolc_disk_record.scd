// imbolc_disk_record SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_disk_record, { |bufnum=0, in=0|
    DiskOut.ar(bufnum, In.ar(in, 2));
}).writeDefFile(dir);
)
