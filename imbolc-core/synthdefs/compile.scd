// Master compile script - loads all SynthDef definition files
(
var dir = thisProcess.nowExecutingPath.dirname;

(dir +/+ "defs/midi.scd").load;
(dir +/+ "defs/oscillators.scd").load;
(dir +/+ "defs/oscillators_complex.scd").load;
(dir +/+ "defs/synthesis.scd").load;
(dir +/+ "defs/physical_models.scd").load;
(dir +/+ "defs/input.scd").load;
(dir +/+ "defs/samplers.scd").load;
(dir +/+ "defs/filters.scd").load;
(dir +/+ "defs/modulation.scd").load;
(dir +/+ "defs/eq.scd").load;
(dir +/+ "defs/effects.scd").load;
(dir +/+ "defs/effects_extended.scd").load;
(dir +/+ "defs/output.scd").load;
(dir +/+ "defs/analysis.scd").load;

"Done! All SynthDefs written to: ".post;
dir.postln;
0.exit;
)
