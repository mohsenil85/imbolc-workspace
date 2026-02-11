/// Pad keyboard for drum machine instruments.
/// Maps keyboard keys to 12 drum pads in a 4x3 grid layout:
///   R T Y U
///   F G H J
///   V B N M
pub struct PadKeyboard {
    active: bool,
}

impl PadKeyboard {
    pub fn new() -> Self {
        Self { active: false }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Map key to pad index (0-11), returns None for non-pad keys
    pub fn key_to_pad(&self, c: char) -> Option<usize> {
        match c {
            'r' => Some(0),
            't' => Some(1),
            'y' => Some(2),
            'u' => Some(3),
            'f' => Some(4),
            'g' => Some(5),
            'h' => Some(6),
            'j' => Some(7),
            'v' => Some(8),
            'b' => Some(9),
            'n' => Some(10),
            'm' => Some(11),
            _ => None,
        }
    }

    /// Status label for rendering
    pub fn status_label(&self) -> String {
        " PADS ".to_string()
    }
}
