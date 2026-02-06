> **Status: Phase 1 Implemented (Import & Playback)**
>
> Custom SynthDef support lives in `imbolc-core/src/state/custom_synthdef.rs`, `imbolc-core/src/scd_parser.rs`, and `imbolc-core/src/audio/engine/voices.rs`/`routing.rs`.
> Phase 1 (Data structures, Parser, Import flow) is complete. Phase 2 (Embedded SCLang Editor) is future work.
> This plan predates the imbolc-core split; paths below use current names.

Current paths:
- `imbolc-core/src/state/instrument.rs`
- `imbolc-core/src/state/instrument_state.rs`
- `imbolc-core/src/dispatch/mod.rs`

# Plan: Custom SynthDef Instruments

## Overview

Add support for user-defined SuperCollider SynthDefs as instrument sources. Users can:
1. Import `.scd` files containing custom SynthDefs
2. Use them as instrument source types
3. Edit dynamically-discovered parameters

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    CUSTOM SYNTHDEF FLOW                       │
│                                                               │
│  1. User selects "Import Custom" in Add Instrument pane       │
│  2. File browser opens → user picks .scd file                 │
│  3. Parse .scd to extract synthdef name + params              │
│  4. Compile via sclang → generates .scsyndef                  │
│  5. Copy to ~/.config/imbolc/synthdefs/                       │
│  6. Register in CustomSynthDefRegistry                        │
│  7. Available as source type for new instruments              │
└──────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Data Structures

### 1.1 Custom SynthDef Registry

**`src/state/custom_synthdef.rs`** (new file):
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub type CustomSynthDefId = u32;

/// Specification for a parameter extracted from .scd file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSpec {
    pub name: String,
    pub default: f32,
    pub min: f32,
    pub max: f32,
}

/// A user-imported custom SynthDef
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSynthDef {
    pub id: CustomSynthDefId,
    pub name: String,              // Display name (derived from synthdef name)
    pub synthdef_name: String,     // SuperCollider name (e.g., "my_bass")
    pub source_path: PathBuf,      // Original .scd file path
    pub params: Vec<ParamSpec>,    // Extracted parameters
}

/// Registry of all custom synthdefs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomSynthDefRegistry {
    pub synthdefs: Vec<CustomSynthDef>,
    pub next_id: CustomSynthDefId,
}

impl CustomSynthDefRegistry {
    pub fn add(&mut self, synthdef: CustomSynthDef) -> CustomSynthDefId { ... }
    pub fn get(&self, id: CustomSynthDefId) -> Option<&CustomSynthDef> { ... }
    pub fn remove(&mut self, id: CustomSynthDefId) { ... }
}
```

### 1.2 SourceType Extension

**`imbolc-core/src/state/instrument.rs`** modifications:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    Saw,
    Sin,
    Sqr,
    Tri,
    AudioIn,
    Sampler,
    Custom(CustomSynthDefId),  // NEW
}

impl SourceType {
    // Change return type to handle dynamic strings
    pub fn synth_def_name_with_registry(&self, registry: &CustomSynthDefRegistry) -> String {
        match self {
            SourceType::Saw => "imbolc_saw".to_string(),
            // ... other built-ins
            SourceType::Custom(id) => {
                registry.get(*id)
                    .map(|s| s.synthdef_name.clone())
                    .unwrap_or_else(|| "imbolc_saw".to_string())
            }
        }
    }

    // New method for custom params
    pub fn default_params(&self, registry: &CustomSynthDefRegistry) -> Vec<Param> {
        match self {
            // ... built-in types return hardcoded params
            SourceType::Custom(id) => {
                registry.get(*id)
                    .map(|s| s.params.iter().map(|p| Param {
                        name: p.name.clone(),
                        value: ParamValue::Float(p.default),
                        min: p.min,
                        max: p.max,
                    }).collect())
                    .unwrap_or_default()
            }
        }
    }
}
```

### 1.3 InstrumentState Integration

**`imbolc-core/src/state/instrument_state.rs`** additions:
```rust
pub struct InstrumentState {
    // ... existing fields
    pub custom_synthdefs: CustomSynthDefRegistry,  // NEW
}
```

---

## Phase 2: SCD File Parser

### 2.1 Parser Module

**`src/scd_parser.rs`** (new file):
```rust
use regex::Regex;

/// Parsed result from an .scd file
pub struct ParsedSynthDef {
    pub name: String,
    pub params: Vec<(String, f32)>,  // (name, default)
}

/// Internal params to filter out (not user-editable)
const INTERNAL_PARAMS: &[&str] = &[
    "out", "freq_in", "gate_in", "vel_in",
    "attack", "decay", "sustain", "release"  // ADSR handled by instrument
];

pub fn parse_scd_file(content: &str) -> Result<ParsedSynthDef, String> {
    // Find SynthDef name: SynthDef(\name, ... or SynthDef("name", ...
    let name_re = Regex::new(r"SynthDef\s*\(\s*[\\\"](\w+)").unwrap();
    let name = name_re.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or("Could not find SynthDef name")?;

    // Find args: { |arg1=val1, arg2=val2, ...|
    let args_re = Regex::new(r"\{\s*\|([^|]+)\|`).unwrap();
    let args_str = args_re.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .ok_or("Could not find SynthDef arguments")?;

    // Parse individual args: name=default or just name
    let param_re = Regex::new(r"(\w+)\s*=\s*\(?\s*(-?[\d.]+)").unwrap();
    let params: Vec<(String, f32)> = param_re.captures_iter(args_str)
        .filter_map(|c| {
            let name = c.get(1)?.as_str().to_string();
            let default: f32 = c.get(2)?.as_str().parse().ok()?;
            // Filter out internal params
            if INTERNAL_PARAMS.contains(&name.as_str()) {
                None
            } else {
                Some((name, default))
            }
        })
        .collect();

    Ok(ParsedSynthDef { name, params })
}

/// Infer min/max from param name and default
pub fn infer_param_range(name: &str, default: f32) -> (f32, f32) {
    match name.to_lowercase().as_str() {
        n if n.contains("freq") => (20.0, 20000.0),
        n if n.contains("amp") || n.contains("level") || n.contains("mix") => (0.0, 1.0),
        n if n.contains("rate") => (0.1, 10.0),
        n if n.contains("time") || n.contains("delay") => (0.0, 2.0),
        n if n.contains("pan") => (-1.0, 1.0),
        n if n.contains("cutoff") => (20.0, 20000.0),
        n if n.contains("resonance") || n.contains("res") => (0.0, 1.0),
        _ => {
            // Generic: ±10x default, or 0-1 if default is in that range
            if default >= 0.0 && default <= 1.0 {
                (0.0, 1.0)
            } else {
                (default * 0.1, default * 10.0)
            }
        }
    }
}
```

### 2.2 Dependencies

**`Cargo.toml`** addition:
```toml
regex = "1"
```

---

## Phase 3: File Browser Pane

### 3.1 File Browser

**`src/panes/file_browser_pane.rs`** (new file):
```rust
pub struct FileBrowserPane {
    keymap: Keymap,
    current_dir: PathBuf,
    entries: Vec<DirEntry>,
    selected: usize,
    filter_extension: Option<String>,  // e.g., "scd"
    on_select_action: FileSelectAction,
}

pub enum FileSelectAction {
    ImportCustomSynthDef,
    // Future: LoadSample, SaveProject, etc.
}

struct DirEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

impl FileBrowserPane {
    pub fn new() -> Self { ... }

    pub fn open_for(&mut self, action: FileSelectAction, start_dir: Option<PathBuf>) {
        self.on_select_action = action;
        self.filter_extension = match action {
            FileSelectAction::ImportCustomSynthDef => Some("scd".to_string()),
        };
        self.current_dir = start_dir.unwrap_or_else(|| {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
        });
        self.refresh_entries();
    }

    fn refresh_entries(&mut self) { ... }
}

impl Pane for FileBrowserPane {
    fn id(&self) -> &'static str { "file_browser" }

    fn handle_input(&mut self, event: InputEvent) -> Action {
        match self.keymap.lookup(&event) {
            Some("select") => {
                if let Some(entry) = self.entries.get(self.selected) {
                    if entry.is_dir {
                        self.current_dir = entry.path.clone();
                        self.refresh_entries();
                        Action::None
                    } else {
                        // File selected
                        match self.on_select_action {
                            FileSelectAction::ImportCustomSynthDef => {
                                Action::ImportCustomSynthDef(entry.path.clone())
                            }
                        }
                    }
                } else {
                    Action::None
                }
            }
            Some("parent") => {
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.refresh_entries();
                }
                Action::None
            }
            Some("cancel") => Action::SwitchPane("add"),
            Some("next") => { self.selected = (self.selected + 1).min(self.entries.len().saturating_sub(1)); Action::None }
            Some("prev") => { self.selected = self.selected.saturating_sub(1); Action::None }
            _ => Action::None
        }
    }

    fn render(&self, g: &mut dyn Graphics) {
        // Centered box showing:
        // - Current path at top
        // - List of directories (with /) and files
        // - Selected entry highlighted
        // - Help text at bottom
    }
}
```

### 3.2 Keybindings
- `Up/Down` or `j/k`: Navigate
- `Enter`: Select file / enter directory
- `Backspace` or `h`: Go to parent directory
- `Escape`: Cancel
- `~`: Go to home directory

---

## Phase 4: Import Flow

### 4.1 New Actions

**`src/ui/pane.rs`** additions:
```rust
pub enum Action {
    // ... existing
    OpenFileBrowser(FileSelectAction),
    ImportCustomSynthDef(PathBuf),
}
```

### 4.2 Import Handler

**`src/dispatch.rs`** additions:
```rust
Action::ImportCustomSynthDef(path) => {
    // 1. Read file
    let content = std::fs::read_to_string(&path)?;

    // 2. Parse
    let parsed = scd_parser::parse_scd_file(&content)?;

    // 3. Compile via sclang
    let synthdefs_dir = config_dir().join("synthdefs");
    std::fs::create_dir_all(&synthdefs_dir)?;

    // Run: sclang -e "... SynthDef code ...writeDefFile(dir)"
    // Or copy .scd and run compile.scd style

    // 4. Create CustomSynthDef entry
    let params: Vec<ParamSpec> = parsed.params.iter().map(|(name, default)| {
        let (min, max) = infer_param_range(name, *default);
        ParamSpec { name: name.clone(), default: *default, min, max }
    }).collect();

    let custom = CustomSynthDef {
        id: state.custom_synthdefs.next_id,
        name: parsed.name.clone(),
        synthdef_name: parsed.name.clone(),
        source_path: path,
        params,
    };

    // 5. Register
    state.custom_synthdefs.add(custom);

    // 6. Switch back to add pane
    Action::SwitchPane("add")
}
```

### 4.3 SynthDef Compilation

Run sclang to compile the .scd file:
```rust
fn compile_synthdef(scd_path: &Path, output_dir: &Path) -> Result<(), String> {
    let status = std::process::Command::new("sclang")
        .arg("-e")
        .arg(format!(
            "var dir = \"{}\"; load(\"{}\"); 0.exit;",
            output_dir.display(),
            scd_path.display()
        ))
        .status()
        .map_err(|e| format!("Failed to run sclang: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("sclang compilation failed".to_string())
    }
}
```

---

## Phase 5: AddPane Integration

### 5.1 Show Custom Types in Add Menu

**`src/panes/add_pane.rs`** modifications:
```rust
impl AddPane {
    fn build_options(&self, registry: &CustomSynthDefRegistry) -> Vec<AddOption> {
        let mut options = vec![
            // Built-in types
            AddOption::Source(SourceType::Saw),
            AddOption::Source(SourceType::Sin),
            // ... etc
        ];

        // Add separator
        options.push(AddOption::Separator("── Custom ──"));

        // Add custom types
        for synth in &registry.synthdefs {
            options.push(AddOption::Source(SourceType::Custom(synth.id)));
        }

        // Add import option
        options.push(AddOption::ImportCustom);

        options
    }
}

enum AddOption {
    Source(SourceType),
    Separator(&'static str),
    ImportCustom,
}
```

### 5.2 Handle Import Selection

```rust
fn handle_input(&mut self, event: InputEvent) -> Action {
    match self.keymap.lookup(&event) {
        Some("select") => {
            match &self.options[self.selected] {
                AddOption::Source(source) => Action::Instrument(InstrumentAction::Add(*source)),
                AddOption::ImportCustom => Action::OpenFileBrowser(FileSelectAction::ImportCustomSynthDef),
                AddOption::Separator(_) => Action::None,
            }
        }
        // ...
    }
}
```

---

## Phase 6: Audio Engine Integration

### 6.1 Load Custom SynthDefs

**`src/audio/engine.rs`** modifications:
```rust
pub fn load_custom_synthdefs(&mut self, registry: &CustomSynthDefRegistry) -> Result<(), String> {
    let synthdefs_dir = config_dir().join("synthdefs");

    for synth in &registry.synthdefs {
        let scsyndef_path = synthdefs_dir.join(format!("{}.scsyndef", synth.synthdef_name));
        if scsyndef_path.exists() {
            self.load_synthdef(&scsyndef_path)?;
        }
    }
    Ok(())
}
```

### 6.2 Spawn Custom Voices

The existing `spawn_voice` method already handles this because:
- It gets synthdef name via `SourceType::synth_def_name_with_registry(registry)`
- It passes all `source_params` to SuperCollider
- Custom params are stored in `instrument.source_params`

Only change needed: use `synth_def_name_with_registry()` where synthdef names are resolved.

---

## Phase 7: Persistence

### 7.1 SQLite Tables

**`src/state/persistence.rs`** additions:
```sql
CREATE TABLE IF NOT EXISTS custom_synthdefs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    synthdef_name TEXT NOT NULL,
    source_path TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS custom_synthdef_params (
    synthdef_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    default_val REAL NOT NULL,
    min_val REAL NOT NULL,
    max_val REAL NOT NULL,
    PRIMARY KEY (synthdef_id, position),
    FOREIGN KEY (synthdef_id) REFERENCES custom_synthdefs(id)
);
```

### 7.2 Save/Load

Add save/load functions for `CustomSynthDefRegistry` in persistence.rs.

---

## Files to Modify/Create

| File |
|------|
| `src/state/custom_synthdef.rs` | CREATE | CustomSynthDef, ParamSpec, Registry types |
| `src/scd_parser.rs` | CREATE | Parse .scd files for name and params |
| `src/panes/file_browser_pane.rs` | CREATE | File selection UI |
| `imbolc-core/src/state/instrument.rs` | MODIFY | Add SourceType::Custom variant |
| `imbolc-core/src/state/instrument_state.rs` | MODIFY | Add custom_synthdefs registry |
| `src/state/mod.rs` | MODIFY | Export new modules |
| `src/panes/add_pane.rs` | MODIFY | Show custom types + import option |
| `src/panes/mod.rs` | MODIFY | Export FileBrowserPane |
| `src/ui/pane.rs` | MODIFY | Add new Action variants |
| `src/dispatch.rs` | MODIFY | Handle import action |
| `src/audio/engine.rs` | MODIFY | Pass registry to synth_def_name_with_registry calls |
| `src/main.rs` | MODIFY | Register FileBrowserPane |
| `src/state/persistence.rs` | MODIFY | Save/load custom synthdefs |
| `Cargo.toml` | MODIFY | Add regex dependency |

---

## Implementation Order

1. **Data structures**: custom_synthdef.rs, SourceType::Custom variant
2. **SCD parser**: scd_parser.rs with regex parsing
3. **File browser pane**: Basic directory listing UI
4. **Actions + dispatch**: Wire up import flow
5. **AddPane integration**: Show custom types in menu
6. **Audio engine**: Pass registry to synth_def_name_with_registry
7. **Persistence**: Save/load custom synthdefs

---

## Verification

### Phase 1 (Import)
- [ ] `cargo build` succeeds
- [ ] File browser opens and navigates directories
- [ ] Selecting .scd file parses name and params correctly
- [ ] Compiled .scsyndef appears in config dir
- [ ] Custom type appears in Add Instrument menu
- [ ] Creating instrument with custom type works
- [ ] Custom params appear in instrument editor
- [ ] Playing notes triggers custom synthdef
- [ ] Save/load preserves custom synthdefs

### Phase 2 (Editor)
- [ ] Editor pane opens with syntax highlighting
- [ ] Readline keybindings work (C-a, C-e, C-k, C-y, etc.)
- [ ] C-x C-s saves file
- [ ] C-c C-c compiles and shows errors/success
- [ ] Tree-sitter highlights keywords, strings, numbers, symbols
- [ ] Undo/redo works (C-/ or C-_)
- [ ] Mark and region work (C-Space, C-w)
- [ ] Line numbers displayed correctly
- [ ] Cursor position shown in status bar

---

---

# Phase 2: Embedded SCLang Editor

## Overview

A full-featured code editor pane for writing and editing SuperCollider SynthDefs directly in imbolc. Features:
- Tree-sitter syntax highlighting for SCLang
- Readline/Emacs-style keybindings
- Live compilation and error feedback
- Parameter extraction on save

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    SCLANG EDITOR PANE                         │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ SynthDef(\*my_synth, { |out=0, freq=440, amp=0.5|       │  │
│  │     var sig = SinOsc.ar(freq) * amp;                   │  │
│  │     Out.ar(out, sig ! 2);                              │  │
│  }).writeDefFile(dir);                                   │  │
│  └────────────────────────────────────────────────────────┘  │
│  Line 3, Col 12 | SCLang | Modified                          │
│  [C-x C-s] Save  [C-x C-c] Close  [C-c C-c] Compile          │
└──────────────────────────────────────────────────────────────┘
```

---

## Editor Core

### 2.1 Text Buffer

**`src/editor/buffer.rs`** (new file):
```rust
pub struct TextBuffer {
    lines: Vec<String>,
    cursor: Position,
    selection: Option<Selection>,
    modified: bool,
    undo_stack: Vec<Edit>,
    redo_stack: Vec<Edit>,
}

#[derive(Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

pub struct Selection {
    pub start: Position,
    pub end: Position,
}

pub enum Edit {
    Insert { pos: Position, text: String },
    Delete { pos: Position, text: String },
}

impl TextBuffer {
    pub fn insert_char(&mut self, ch: char) { ... }
    pub fn delete_char(&mut self) { ... }
    pub fn delete_backward(&mut self) { ... }
    pub fn kill_line(&mut self) { ... }  // C-k
    pub fn kill_region(&mut self) { ... }  // C-w
    pub fn yank(&mut self) { ... }  // C-y
    pub fn undo(&mut self) { ... }  // C-/
    pub fn redo(&mut self) { ... }

    // Movement
    pub fn move_forward_char(&mut self) { ... }  // C-f
    pub fn move_backward_char(&mut self) { ... }  // C-b
    pub fn move_forward_word(&mut self) { ... }  // M-f
    pub fn move_backward_word(&mut self) { ... }  // M-b
    pub fn move_beginning_of_line(&mut self) { ... }  // C-a
    pub fn move_end_of_line(&mut self) { ... }  // C-e
    pub fn move_next_line(&mut self) { ... }  // C-n
    pub fn move_prev_line(&mut self) { ... }  // C-p

    // Selection
    pub fn set_mark(&mut self) { ... }  // C-Space
    pub fn exchange_point_and_mark(&mut self) { ... }  // C-x C-x
}
```

### 2.2 Tree-sitter Integration

**`Cargo.toml`** additions:
```toml
tree-sitter = "0.20"
tree-sitter-supercollider = { git = "https://github.com/madskjeldgaard/tree-sitter-supercollider" }
```

**`src/editor/highlighting.rs`** (new file):
```rust
use tree_sitter::{Language, Parser, Tree};
use tree_sitter_highlight::{Highlighter, HighlightConfiguration, HighlightEvent};

pub struct SyntaxHighlighter {
    parser: Parser,
    config: HighlightConfiguration,
}

pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub style: HighlightStyle,
}

#[derive(Clone, Copy)]
pub enum HighlightStyle {
    Keyword,      // SynthDef, var, Out, etc.
    String,       // "strings"
    Number,       // 440, 0.5
    Symbol,       // \symbol
    Comment,      // // comment
    Function,     // .ar, .kr, .new
    Operator,     // +, -, *, /, =
    UGen,         // SinOsc, Saw, LPF, etc.
    Argument,     // |arg=val|
    Default,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_supercollider::language()).unwrap();

        let config = HighlightConfiguration::new(
            tree_sitter_supercollider::language(),
            tree_sitter_supercollider::HIGHLIGHTS_QUERY,
            "", // injections
            "", // locals
        ).unwrap();

        Self { parser, config }
    }

    pub fn highlight(&mut self, source: &str) -> Vec<HighlightSpan> {
        // Parse and return spans with styles
    }
}
```

### 2.3 Color Scheme

**`src/editor/theme.rs`** (new file):
```rust
use crate::ui::Color;

pub fn highlight_color(style: HighlightStyle) -> Color {
    match style {
        HighlightStyle::Keyword => Color::new(198, 120, 221),   // Purple
        HighlightStyle::String => Color::new(152, 195, 121),    // Green
        HighlightStyle::Number => Color::new(209, 154, 102),    // Orange
        HighlightStyle::Symbol => Color::new(86, 182, 194),     // Cyan
        HighlightStyle::Comment => Color::new(92, 99, 112),     // Gray
        HighlightStyle::Function => Color::new(97, 175, 239),   // Blue
        HighlightStyle::Operator => Color::new(171, 178, 191),  // Light gray
        HighlightStyle::UGen => Color::new(224, 108, 117),      // Red
        HighlightStyle::Argument => Color::new(229, 192, 123),  // Yellow
        HighlightStyle::Default => Color::WHITE,
    }
}
```

---

## Editor Pane

### 2.4 SCLang Editor Pane

**`src/panes/sclang_editor_pane.rs`** (new file):
```rust
pub struct SclangEditorPane {
    buffer: TextBuffer,
    highlighter: SyntaxHighlighter,
    highlight_cache: Vec<HighlightSpan>,
    file_path: Option<PathBuf>,
    scroll_offset: usize,
    kill_ring: Vec<String>,  // For yank
    mark_active: bool,
    mode: EditorMode,
    compile_errors: Vec<CompileError>,
}

pub enum EditorMode {
    Normal,
    MiniBuffer { prompt: String, input: String, on_confirm: MiniBufferAction },
}

pub enum MiniBufferAction {
    SaveAs,
    GotoLine,
    Search,
}

pub struct CompileError {
    pub line: usize,
    pub message: String,
}
```

### 2.5 Readline Keybindings

```rust
impl Pane for SclangEditorPane {
    fn handle_input(&mut self, event: InputEvent) -> Action {
        // Check for chord sequences first (C-x prefix)
        if self.pending_prefix == Some(Prefix::CtrlX) {
            self.pending_prefix = None;
            return match event.key {
                KeyCode::Char('s') if event.modifiers.ctrl => self.save(),
                KeyCode::Char('c') if event.modifiers.ctrl => Action::SwitchPane("instrument"),
                KeyCode::Char('f') if event.modifiers.ctrl => self.open_file(),
                KeyCode::Char('x') if event.modifiers.ctrl => self.exchange_point_and_mark(),
                _ => Action::None,
            };
        }

        match event.key {
            // C-x prefix
            KeyCode::Char('x') if event.modifiers.ctrl => {
                self.pending_prefix = Some(Prefix::CtrlX);
                Action::None
            }

            // Movement
            KeyCode::Char('f') if event.modifiers.ctrl => { self.buffer.move_forward_char(); Action::None }
            KeyCode::Char('b') if event.modifiers.ctrl => { self.buffer.move_backward_char(); Action::None }
            KeyCode::Char('f') if event.modifiers.alt => { self.buffer.move_forward_word(); Action::None }
            KeyCode::Char('b') if event.modifiers.alt => { self.buffer.move_backward_word(); Action::None }
            KeyCode::Char('a') if event.modifiers.ctrl => { self.buffer.move_beginning_of_line(); Action::None }
            KeyCode::Char('e') if event.modifiers.ctrl => { self.buffer.move_end_of_line(); Action::None }
            KeyCode::Char('n') if event.modifiers.ctrl => { self.buffer.move_next_line(); Action::None }
            KeyCode::Char('p') if event.modifiers.ctrl => { self.buffer.move_prev_line(); Action::None }
            KeyCode::Char('v') if event.modifiers.ctrl => { self.page_down(); Action::None }
            KeyCode::Char('v') if event.modifiers.alt => { self.page_up(); Action::None }

            // Editing
            KeyCode::Char('d') if event.modifiers.ctrl => { self.buffer.delete_char(); Action::None }
            KeyCode::Char('k') if event.modifiers.ctrl => { self.kill_line(); Action::None }
            KeyCode::Char('w') if event.modifiers.ctrl => { self.kill_region(); Action::None }
            KeyCode::Char('y') if event.modifiers.ctrl => { self.yank(); Action::None }
            KeyCode::Char('y') if event.modifiers.alt => { self.yank_pop(); Action::None }
            KeyCode::Char('/') if event.modifiers.ctrl => { self.buffer.undo(); Action::None }
            KeyCode::Char('_') if event.modifiers.ctrl => { self.buffer.undo(); Action::None }

            // Selection
            KeyCode::Char(' ') if event.modifiers.ctrl => { self.buffer.set_mark(); Action::None }

            // Compile
            KeyCode::Char('c') if event.modifiers.ctrl => {
                self.pending_prefix = Some(Prefix::CtrlC);
                Action::None
            }

            // Arrow keys (also work)
            KeyCode::Up => { self.buffer.move_prev_line(); Action::None }
            KeyCode::Down => { self.buffer.move_next_line(); Action::None }
            KeyCode::Left => { self.buffer.move_backward_char(); Action::None }
            KeyCode::Right => { self.buffer.move_forward_char(); Action::None }

            // Regular typing
            KeyCode::Char(c) => { self.buffer.insert_char(c); self.rehighlight(); Action::None }
            KeyCode::Enter => { self.buffer.insert_char('\n'); self.rehighlight(); Action::None }
            KeyCode::Backspace => { self.buffer.delete_backward(); self.rehighlight(); Action::None }
            KeyCode::Tab => { self.buffer.insert_char('\t'); Action::None }

            _ => Action::None
        }
    }
}
```

### 2.6 Compile Integration

```rust
impl SclangEditorPane {
    fn compile(&mut self) -> Action {
        // Save to temp file
        let temp_path = std::env::temp_dir().join("imbolc_compile.scd");
        std::fs::write(&temp_path, self.buffer.content()).ok();

        // Run sclang
        let output = std::process::Command::new("sclang")
            .arg(&temp_path)
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    self.compile_errors.clear();
                    // Parse the synthdef and register it
                    Action::CompileSynthDefSuccess(self.buffer.content())
                } else {
                    // Parse errors from stderr
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    self.compile_errors = parse_sclang_errors(&stderr);
                    Action::None
                }
            }
            Err(e) => {
                self.compile_errors = vec![CompileError {
                    line: 0,
                    message: format!("Failed to run sclang: {}", e),
                }];
                Action::None
            }
        }
    }
}
```

---

## Rendering

### 2.7 Editor Rendering

```rust
fn render(&self, g: &mut dyn Graphics) {
    let (width, height) = g.size();
    let rect = Rect::new(0, 0, width, height);  // Full screen for editor

    // Title bar
    g.set_style(Style::new().fg(Color::BLACK).bg(Color::CYAN));
    let title = self.file_path.as_ref()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .unwrap_or_else(|| "[New SynthDef]".to_string());
    let modified = if self.buffer.modified { " [+]" } else { "" };
    g.put_str(0, 0, &format!(" {} {}", title, modified));

    // Line numbers gutter
    let gutter_width = 4;
    let visible_lines = (height - 3) as usize;  // -3 for title, status, help

    for (i, line_num) in (self.scroll_offset..self.scroll_offset + visible_lines).enumerate() {
        let y = i as u16 + 1;

        // Line number
        g.set_style(Style::new().fg(Color::DARK_GRAY));
        if line_num < self.buffer.line_count() {
            g.put_str(0, y, &format!("{:>3} ", line_num + 1));
        }

        // Line content with syntax highlighting
        if let Some(line) = self.buffer.line(line_num) {
            let highlights = self.highlights_for_line(line_num);
            self.render_highlighted_line(g, gutter_width, y, line, &highlights);
        }
    }

    // Cursor
    let cursor_y = (self.buffer.cursor.line - self.scroll_offset) as u16 + 1;
    let cursor_x = gutter_width + self.buffer.cursor.col as u16;
    g.set_cursor(cursor_x, cursor_y);

    // Status line
    let status_y = height - 2;
    g.set_style(Style::new().fg(Color::BLACK).bg(Color::DARK_GRAY));
    g.put_str(0, status_y, &format!(
        " L{}, C{} | SCLang | {} ",
        self.buffer.cursor.line + 1,
        self.buffer.cursor.col + 1,
        if self.buffer.modified { "Modified" } else { "Saved" }
    ));

    // Compile errors (if any)
    if !self.compile_errors.is_empty() {
        g.set_style(Style::new().fg(Color::RED));
        g.put_str(30, status_y, &format!(" | {} error(s)", self.compile_errors.len()));
    }

    // Help line
    g.set_style(Style::new().fg(Color::DARK_GRAY));
    g.put_str(0, height - 1, " C-x C-s: Save | C-c C-c: Compile | C-x C-c: Close | C-g: Cancel ");
}
```

---

## Files to Add (Phase 2)

| File |
|------|
| `src/editor/mod.rs` | Editor module root |
| `src/editor/buffer.rs` | Text buffer with undo/redo |
| `src/editor/highlighting.rs` | Tree-sitter syntax highlighting |
| `src/editor/theme.rs` | Color scheme for SCLang |
| `src/panes/sclang_editor_pane.rs` | Full editor pane |

## Dependencies (Phase 2)

```toml
tree-sitter = "0.20"
tree-sitter-supercollider = { git = "https://github.com/madskjeldgaard/tree-sitter-supercollider" }
```

---

## SynthDef File Organization

**Hard rule: One SynthDef per file.** Built-in SynthDefs live in `imbolc-core/synthdefs/defs/` organized by category:

```
defs/
├── oscillators/      Basic waveforms (saw, sin, sqr, tri, pulse, noise)
├── synthesis/        Synthesis techniques (fm, granular, additive, wavetable)
├── physical_models/  Physical modeling (bowed, blown, guitar, marimba)
├── drums/            Drum sounds (kick, snare, hihat, clap, tom)
├── classic_synths/   Classic emulations (organ, epiano, brass_stab, strings)
├── filters/          Filter effects (lpf, hpf, bpf, notch, comb, allpass)
├── effects/          Audio effects (delay, reverb, chorus, distortion)
├── modulation/       Modulation sources (lfo, adsr)
├── eq/               EQ processors
├── input/            Input sources (audio_in, bus_in)
├── output/           Output routing (output, send, bus_out)
├── samplers/         Sample playback
├── analysis/         Metering and analysis
└── midi/             MIDI output
```

Each file is named after its SynthDef (e.g., `imbolc_kick.scd` contains `\imbolc_kick`).

## Custom SynthDef Convention

Users should follow this pattern for compatibility:

```supercollider
// imbolc_my_custom_synth SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_my_custom_synth, {
    // Required inputs (handled by instrument system)
    |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    // Standard envelope (handled by instrument ADSR section)
    attack=0.01, decay=0.1, sustain=0.7, release=0.3,
    // Custom params (will be editable)
    my_param=0.5, another_param=1000|

    // Standard input handling
    var freqSig = Select.kr(freq_in >= 0, [440, In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);

    // Your synthesis code here
    var sig = SinOsc.ar(freqSig * my_param) * velSig;

    // Standard envelope
    var env = EnvGen.kr(Env.adsr(attack, decay, sustain, release), gateSig);

    Out.ar(out, (sig * env) ! 2);
}).writeDefFile(dir);
)
```

### Important: var Declaration Order

SuperCollider requires all `var` declarations before any statements:

```supercollider
// WRONG - will fail to compile
var sig = SinOsc.ar(freq);
sig = sig * 0.5;           // statement
var env = EnvGen.kr(...);  // ERROR: var after statement

// CORRECT
var sig = SinOsc.ar(freq);
var env;                   // declare first
sig = sig * 0.5;
env = EnvGen.kr(...);      // assign later
```
