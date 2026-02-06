pub use crate::state::KeyboardLayout;

/// Translate a key character from the configured layout to QWERTY physical position.
pub fn translate_key(c: char, layout: KeyboardLayout) -> char {
    match layout {
        KeyboardLayout::Qwerty => c,
        KeyboardLayout::Colemak => colemak_to_qwerty(c),
    }
}

fn colemak_to_qwerty(c: char) -> char {
    match c {
        // top row
        'f' => 'e', 'p' => 'r', 'g' => 't', 'j' => 'y',
        'l' => 'u', 'u' => 'i', 'y' => 'o', ';' => 'p',
        // home row
        'r' => 's', 's' => 'd', 't' => 'f', 'd' => 'g',
        'n' => 'j', 'e' => 'k', 'i' => 'l', 'o' => ';',
        // bottom row
        'k' => 'n',
        // uppercase (Stradella shifted rows)
        'F' => 'E', 'P' => 'R', 'G' => 'T', 'J' => 'Y',
        'L' => 'U', 'U' => 'I', 'Y' => 'O', ':' => 'P',
        'R' => 'S', 'S' => 'D', 'T' => 'F', 'D' => 'G',
        'N' => 'J', 'E' => 'K', 'I' => 'L', 'O' => ':',
        'K' => 'N',
        // unchanged keys pass through
        other => other,
    }
}

/// Piano keyboard layout starting note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PianoLayout {
    C,
    A,
    Stradella,
}

/// Stradella bass row types.
enum StradellaRow {
    CounterBass,
    Bass,
    Major,
    Minor,
    Dom7,
    Dim7,
}

/// Shared piano keyboard state and key-to-pitch mapping.
///
/// Used by InstrumentPane, PianoRollPane, and InstrumentEditPane.
///
/// Available methods:
/// - `activate()` / `deactivate()` / `is_active()` — toggle piano mode
/// - `key_to_pitch(char) -> Option<u8>` — map keyboard char to MIDI pitch
/// - `handle_escape() -> bool` — cycle C→A→off, returns true if deactivated
/// - `octave_up()` / `octave_down()` — change octave (returns new octave)
/// - `octave()` — current octave
/// - `status_label() -> String` — e.g. "PIANO C4"
///
/// No `set_layout()` or `set_octave()` methods exist.
pub struct PianoKeyboard {
    active: bool,
    octave: i8,
    layout: PianoLayout,
}

impl PianoKeyboard {
    pub fn new() -> Self {
        Self {
            active: false,
            octave: 4,
            layout: PianoLayout::C,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.layout = PianoLayout::C;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn octave(&self) -> i8 {
        self.octave
    }

    /// Cycle layout C→A→Stradella→off. Returns true if piano mode was deactivated.
    pub fn handle_escape(&mut self) -> bool {
        match self.layout {
            PianoLayout::C => {
                self.layout = PianoLayout::A;
                false
            }
            PianoLayout::A => {
                self.layout = PianoLayout::Stradella;
                false
            }
            PianoLayout::Stradella => {
                self.active = false;
                true
            }
        }
    }

    /// Decrease octave. Returns true if changed.
    pub fn octave_down(&mut self) -> bool {
        if self.octave > -1 {
            self.octave -= 1;
            true
        } else {
            false
        }
    }

    /// Increase octave. Returns true if changed.
    pub fn octave_up(&mut self) -> bool {
        if self.octave < 9 {
            self.octave += 1;
            true
        } else {
            false
        }
    }

    /// Status label for rendering, e.g. "PIANO C4" or "BASS 4".
    pub fn status_label(&self) -> String {
        match self.layout {
            PianoLayout::C => format!(" PIANO C{} ", self.octave),
            PianoLayout::A => format!(" PIANO A{} ", self.octave),
            PianoLayout::Stradella => format!(" BASS {} ", self.octave),
        }
    }

    /// Convert a keyboard character to a MIDI pitch using current octave and layout.
    /// Returns None for Stradella layout (use key_to_pitches instead).
    pub fn key_to_pitch(&self, key: char) -> Option<u8> {
        let offset = match self.layout {
            PianoLayout::C => Self::key_to_offset_c(key),
            PianoLayout::A => Self::key_to_offset_a(key),
            PianoLayout::Stradella => return None,
        };
        offset.map(|off| {
            let base = match self.layout {
                PianoLayout::C => (self.octave as i16 + 1) * 12,
                PianoLayout::A => (self.octave as i16 + 1) * 12 - 3,
                PianoLayout::Stradella => unreachable!(),
            };
            (base + off as i16).clamp(0, 127) as u8
        })
    }

    /// Convert a keyboard character to MIDI pitches using current layout.
    /// For C/A layouts, returns a single pitch. For Stradella, returns chord pitches.
    pub fn key_to_pitches(&self, key: char) -> Option<Vec<u8>> {
        match self.layout {
            PianoLayout::C | PianoLayout::A => {
                self.key_to_pitch(key).map(|p| vec![p])
            }
            PianoLayout::Stradella => {
                self.stradella_pitches(key)
            }
        }
    }

    /// Whether the current layout is Stradella (shift selects rows, not velocity).
    #[allow(dead_code)]
    pub fn is_stradella(&self) -> bool {
        self.layout == PianoLayout::Stradella
    }

    /// Map a keyboard character to a MIDI note offset for C layout.
    fn key_to_offset_c(key: char) -> Option<u8> {
        match key {
            'a' => Some(0),   // C
            's' => Some(2),   // D
            'd' => Some(4),   // E
            'f' => Some(5),   // F
            'g' => Some(7),   // G
            'h' => Some(9),   // A
            'j' => Some(11),  // B
            'w' => Some(1),   // C#
            'e' => Some(3),   // D#
            't' => Some(6),   // F#
            'y' => Some(8),   // G#
            'u' => Some(10),  // A#
            'k' => Some(12),  // C (octave up)
            'l' => Some(14),  // D
            ';' => Some(16),  // E
            'o' => Some(13),  // C#
            'p' => Some(15),  // D#
            _ => None,
        }
    }

    /// Map a keyboard character to a MIDI note offset for A layout.
    fn key_to_offset_a(key: char) -> Option<u8> {
        match key {
            'a' => Some(0),   // A
            's' => Some(2),   // B
            'd' => Some(3),   // C
            'f' => Some(5),   // D
            'g' => Some(7),   // E
            'h' => Some(8),   // F
            'j' => Some(10),  // G
            'w' => Some(1),   // A#
            'e' => Some(4),   // C#
            't' => Some(6),   // D#
            'y' => Some(9),   // F#
            'u' => Some(11),  // G#
            'k' => Some(12),  // A (octave up)
            'l' => Some(14),  // B
            ';' => Some(15),  // C
            'o' => Some(13),  // A#
            'p' => Some(16),  // C#
            _ => None,
        }
    }

    /// Build MIDI pitches for a Stradella bass key press.
    fn stradella_pitches(&self, key: char) -> Option<Vec<u8>> {
        let (col, row) = Self::stradella_key_info(key)?;

        // Circle of fifths: Eb Bb F C G D A E B F#
        const FIFTHS: [i16; 10] = [3, 10, 5, 0, 7, 2, 9, 4, 11, 6];
        let root = FIFTHS[col];

        let chord_base = (self.octave as i16 + 1) * 12;
        let bass_base = chord_base - 12;

        let pitches = match row {
            StradellaRow::CounterBass => {
                // Major 3rd above root, bass octave
                vec![(bass_base + root + 4).clamp(0, 127) as u8]
            }
            StradellaRow::Bass => {
                // Root, bass octave
                vec![(bass_base + root).clamp(0, 127) as u8]
            }
            StradellaRow::Major => {
                // Major triad
                vec![
                    (chord_base + root).clamp(0, 127) as u8,
                    (chord_base + root + 4).clamp(0, 127) as u8,
                    (chord_base + root + 7).clamp(0, 127) as u8,
                ]
            }
            StradellaRow::Minor => {
                // Minor triad
                vec![
                    (chord_base + root).clamp(0, 127) as u8,
                    (chord_base + root + 3).clamp(0, 127) as u8,
                    (chord_base + root + 7).clamp(0, 127) as u8,
                ]
            }
            StradellaRow::Dom7 => {
                // Dominant 7th (root, major 3rd, minor 7th)
                vec![
                    (chord_base + root).clamp(0, 127) as u8,
                    (chord_base + root + 4).clamp(0, 127) as u8,
                    (chord_base + root + 10).clamp(0, 127) as u8,
                ]
            }
            StradellaRow::Dim7 => {
                // Diminished 7th (root, minor 3rd, dim 7th dropped octave)
                vec![
                    (chord_base + root).clamp(0, 127) as u8,
                    (chord_base + root + 3).clamp(0, 127) as u8,
                    (chord_base + root - 3).clamp(0, 127) as u8,
                ]
            }
        };

        Some(pitches)
    }

    /// Map a keyboard character to Stradella column index and row type.
    /// 3 physical rows with shift selecting the alternate row:
    /// - QWERTY: unshifted=Dom7, shifted=Dim7
    /// - Home:   unshifted=Major, shifted=Minor
    /// - Bottom: unshifted=Bass,  shifted=CounterBass
    fn stradella_key_info(key: char) -> Option<(usize, StradellaRow)> {
        match key {
            // Bass (bottom row, unshifted)
            'z' => Some((0, StradellaRow::Bass)),
            'x' => Some((1, StradellaRow::Bass)),
            'c' => Some((2, StradellaRow::Bass)),
            'v' => Some((3, StradellaRow::Bass)),
            'b' => Some((4, StradellaRow::Bass)),
            'n' => Some((5, StradellaRow::Bass)),
            'm' => Some((6, StradellaRow::Bass)),
            ',' => Some((7, StradellaRow::Bass)),
            '.' => Some((8, StradellaRow::Bass)),
            '/' => Some((9, StradellaRow::Bass)),

            // CounterBass (bottom row, shifted)
            'Z' => Some((0, StradellaRow::CounterBass)),
            'X' => Some((1, StradellaRow::CounterBass)),
            'C' => Some((2, StradellaRow::CounterBass)),
            'V' => Some((3, StradellaRow::CounterBass)),
            'B' => Some((4, StradellaRow::CounterBass)),
            'N' => Some((5, StradellaRow::CounterBass)),
            'M' => Some((6, StradellaRow::CounterBass)),
            '<' => Some((7, StradellaRow::CounterBass)),
            '>' => Some((8, StradellaRow::CounterBass)),
            '?' => Some((9, StradellaRow::CounterBass)),

            // Major (home row, unshifted)
            'a' => Some((0, StradellaRow::Major)),
            's' => Some((1, StradellaRow::Major)),
            'd' => Some((2, StradellaRow::Major)),
            'f' => Some((3, StradellaRow::Major)),
            'g' => Some((4, StradellaRow::Major)),
            'h' => Some((5, StradellaRow::Major)),
            'j' => Some((6, StradellaRow::Major)),
            'k' => Some((7, StradellaRow::Major)),
            'l' => Some((8, StradellaRow::Major)),
            ';' => Some((9, StradellaRow::Major)),

            // Minor (home row, shifted)
            'A' => Some((0, StradellaRow::Minor)),
            'S' => Some((1, StradellaRow::Minor)),
            'D' => Some((2, StradellaRow::Minor)),
            'F' => Some((3, StradellaRow::Minor)),
            'G' => Some((4, StradellaRow::Minor)),
            'H' => Some((5, StradellaRow::Minor)),
            'J' => Some((6, StradellaRow::Minor)),
            'K' => Some((7, StradellaRow::Minor)),
            'L' => Some((8, StradellaRow::Minor)),
            ':' => Some((9, StradellaRow::Minor)),

            // Dom7 (qwerty row, unshifted)
            'q' => Some((0, StradellaRow::Dom7)),
            'w' => Some((1, StradellaRow::Dom7)),
            'e' => Some((2, StradellaRow::Dom7)),
            'r' => Some((3, StradellaRow::Dom7)),
            't' => Some((4, StradellaRow::Dom7)),
            'y' => Some((5, StradellaRow::Dom7)),
            'u' => Some((6, StradellaRow::Dom7)),
            'i' => Some((7, StradellaRow::Dom7)),
            'o' => Some((8, StradellaRow::Dom7)),
            'p' => Some((9, StradellaRow::Dom7)),

            // Dim7 (qwerty row, shifted)
            'Q' => Some((0, StradellaRow::Dim7)),
            'W' => Some((1, StradellaRow::Dim7)),
            'E' => Some((2, StradellaRow::Dim7)),
            'R' => Some((3, StradellaRow::Dim7)),
            'T' => Some((4, StradellaRow::Dim7)),
            'Y' => Some((5, StradellaRow::Dim7)),
            'U' => Some((6, StradellaRow::Dim7)),
            'I' => Some((7, StradellaRow::Dim7)),
            'O' => Some((8, StradellaRow::Dim7)),
            'P' => Some((9, StradellaRow::Dim7)),

            _ => None,
        }
    }
}
