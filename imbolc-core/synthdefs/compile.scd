// Master compile script - loads all SynthDef definition files from subdirectories
(
var dir = thisProcess.nowExecutingPath.dirname;
var defsDir = dir +/+ "defs";

// Load all .scd files from each subdirectory
var subdirs = [
    "midi",
    "oscillators",
    "synthesis",
    "physical_models",
    "drums",
    "classic_synths",
    "input",
    "samplers",
    "filters",
    "modulation",
    "eq",
    "effects",
    "output",
    "analysis"
];

subdirs.do { |subdir|
    var subPath = defsDir +/+ subdir;
    var files = (subPath +/+ "*.scd").pathMatch;
    files.do { |file|
        file.load;
    };
};

"Done! All SynthDefs written to: ".post;
dir.postln;
0.exit;
)
