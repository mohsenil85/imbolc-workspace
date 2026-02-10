use std::collections::HashMap;
use std::time::{Duration, Instant};

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
/// Sustain tracking:
/// - `key_pressed(char, pitches, timestamp)` — returns Some(pitches) if new press, None if repeat
/// - `check_releases(timestamp)` — returns keys that timed out (need note-off)
/// - `release_all()` — release all active keys, returns all pitches
/// - `has_active_keys()` — whether any keys are currently held
///
/// No `set_layout()` or `set_octave()` methods exist.
pub struct PianoKeyboard {
    active: bool,
    octave: i8,
    layout: PianoLayout,
    // Sustain tracking - supports chords (multiple pitches per key)
    active_keys: HashMap<char, (Vec<u8>, Instant)>,  // char -> (pitches, last_event_time)
    release_timeout: Duration,
}

impl PianoKeyboard {
    pub fn new() -> Self {
        Self {
            active: false,
            octave: 4,
            layout: PianoLayout::C,
            active_keys: HashMap::new(),
            release_timeout: Duration::from_millis(150),
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

    // ── Sustain tracking ──────────────────────────────────────────

    /// Returns Some(pitches) if this is a NEW key press (spawn voices)
    /// Returns None if this is key repeat (sustain, ignore)
    pub fn key_pressed(&mut self, c: char, pitches: Vec<u8>, now: Instant) -> Option<Vec<u8>> {
        if let std::collections::hash_map::Entry::Occupied(mut e) = self.active_keys.entry(c) {
            // Key is already held - just update timestamp (sustain)
            e.insert((pitches, now));
            return None;
        }
        // New press - key wasn't being held
        let result = pitches.clone();
        self.active_keys.insert(c, (pitches, now));
        Some(result)
    }

    /// Check for keys that should be released, returns (char, pitches) pairs
    pub fn check_releases(&mut self, now: Instant) -> Vec<(char, Vec<u8>)> {
        let mut to_release = Vec::new();
        self.active_keys.retain(|&c, (pitches, last_time)| {
            if now.duration_since(*last_time) > self.release_timeout {
                to_release.push((c, pitches.clone()));
                false
            } else {
                true
            }
        });
        to_release
    }

    /// Release all active keys, returns all pitches
    pub fn release_all(&mut self) -> Vec<u8> {
        let pitches: Vec<u8> = self.active_keys.values()
            .flat_map(|(p, _)| p.clone())
            .collect();
        self.active_keys.clear();
        pitches
    }

    /// Check if any keys are currently held
    pub fn has_active_keys(&self) -> bool {
        !self.active_keys.is_empty()
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

    #[cfg(test)]
    pub fn layout(&self) -> PianoLayout {
        self.layout
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let kb = PianoKeyboard::new();
        assert!(!kb.is_active());
        assert_eq!(kb.octave(), 4);
        assert_eq!(kb.layout(), PianoLayout::C);
    }

    #[test]
    fn activate_deactivate() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        assert!(kb.is_active());
        kb.deactivate();
        assert!(!kb.is_active());
    }

    #[test]
    fn handle_escape_cycles_c_a_stradella_off() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        assert_eq!(kb.layout(), PianoLayout::C);

        assert!(!kb.handle_escape()); // C -> A
        assert_eq!(kb.layout(), PianoLayout::A);

        assert!(!kb.handle_escape()); // A -> Stradella
        assert_eq!(kb.layout(), PianoLayout::Stradella);

        assert!(kb.handle_escape()); // Stradella -> off
        assert!(!kb.is_active());
    }

    #[test]
    fn octave_up_clamps_at_9() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        for _ in 0..20 {
            kb.octave_up();
        }
        assert_eq!(kb.octave(), 9);
        assert!(!kb.octave_up()); // at max, returns false
    }

    #[test]
    fn octave_down_clamps_at_neg1() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        for _ in 0..20 {
            kb.octave_down();
        }
        assert_eq!(kb.octave(), -1);
        assert!(!kb.octave_down()); // at min, returns false
    }

    #[test]
    fn status_label_c() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        assert!(kb.status_label().contains("PIANO"));
        assert!(kb.status_label().contains("C4"));
    }

    #[test]
    fn status_label_a() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        kb.handle_escape(); // C -> A
        assert!(kb.status_label().contains("PIANO"));
        assert!(kb.status_label().contains("A4"));
    }

    #[test]
    fn status_label_stradella() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        kb.handle_escape(); // C -> A
        kb.handle_escape(); // A -> Stradella
        assert!(kb.status_label().contains("BASS"));
    }

    #[test]
    fn key_to_pitch_c_layout_a_is_c() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        // 'a' in C layout is C, at octave 4: (4+1)*12 + 0 = 60
        assert_eq!(kb.key_to_pitch('a'), Some(60));
    }

    #[test]
    fn key_to_pitch_c_layout_sharps() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        // 'w' = C# = offset 1 -> 61
        assert_eq!(kb.key_to_pitch('w'), Some(61));
        // 'e' = D# = offset 3 -> 63
        assert_eq!(kb.key_to_pitch('e'), Some(63));
        // 't' = F# = offset 6 -> 66
        assert_eq!(kb.key_to_pitch('t'), Some(66));
    }

    #[test]
    fn key_to_pitch_a_layout_a_is_a() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        kb.handle_escape(); // C -> A
        // 'a' in A layout is offset 0 from A. Base = (4+1)*12 - 3 = 57
        assert_eq!(kb.key_to_pitch('a'), Some(57));
    }

    #[test]
    fn key_to_pitch_unknown_key_none() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        assert_eq!(kb.key_to_pitch('1'), None);
    }

    #[test]
    fn key_to_pitch_stradella_returns_none() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        kb.handle_escape(); // C -> A
        kb.handle_escape(); // A -> Stradella
        assert_eq!(kb.key_to_pitch('a'), None);
    }

    #[test]
    fn key_to_pitches_stradella_bass() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        kb.handle_escape(); // C -> A
        kb.handle_escape(); // A -> Stradella
        // 'c' = Bass, col 2 (F root). Bass = single root note at bass octave.
        let pitches = kb.key_to_pitches('c').unwrap();
        assert_eq!(pitches.len(), 1);
    }

    #[test]
    fn key_to_pitches_stradella_major() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        kb.handle_escape();
        kb.handle_escape();
        // 'd' = Major, col 2 (F root). Major triad = 3 notes.
        let pitches = kb.key_to_pitches('d').unwrap();
        assert_eq!(pitches.len(), 3);
    }

    #[test]
    fn key_to_pitches_stradella_minor() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        kb.handle_escape();
        kb.handle_escape();
        // 'D' (shifted) = Minor, col 2 (F root). Minor triad = 3 notes.
        let pitches = kb.key_to_pitches('D').unwrap();
        assert_eq!(pitches.len(), 3);
    }

    #[test]
    fn key_to_pitches_stradella_dom7() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        kb.handle_escape();
        kb.handle_escape();
        // 'e' = Dom7, col 2 (F root). Dom7 = 3 notes (root, 3rd, 7th).
        let pitches = kb.key_to_pitches('e').unwrap();
        assert_eq!(pitches.len(), 3);
    }

    #[test]
    fn sustain_new_press_returns_pitches() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        let now = Instant::now();
        let result = kb.key_pressed('a', vec![60], now);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![60]);
    }

    #[test]
    fn sustain_repeat_returns_none() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        let now = Instant::now();
        kb.key_pressed('a', vec![60], now);
        // Second press of same key returns None (key repeat)
        let result = kb.key_pressed('a', vec![60], now);
        assert!(result.is_none());
    }

    #[test]
    fn release_all_clears() {
        let mut kb = PianoKeyboard::new();
        kb.activate();
        let now = Instant::now();
        kb.key_pressed('a', vec![60], now);
        kb.key_pressed('s', vec![62], now);
        assert!(kb.has_active_keys());
        let released = kb.release_all();
        assert!(!kb.has_active_keys());
        assert_eq!(released.len(), 2);
    }

    #[test]
    fn colemak_translation() {
        assert_eq!(translate_key('f', KeyboardLayout::Colemak), 'e');
        assert_eq!(translate_key('p', KeyboardLayout::Colemak), 'r');
        assert_eq!(translate_key('r', KeyboardLayout::Colemak), 's');
    }

    #[test]
    fn colemak_unchanged_keys() {
        // Keys not in colemak map should pass through
        assert_eq!(translate_key('a', KeyboardLayout::Colemak), 'a');
        assert_eq!(translate_key('q', KeyboardLayout::Colemak), 'q');
        assert_eq!(translate_key('z', KeyboardLayout::Colemak), 'z');
    }
}
