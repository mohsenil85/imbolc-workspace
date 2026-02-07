/// Ticks per grid cell based on zoom level.
///
/// Zoom levels map to musical subdivisions at 480 ticks per beat:
/// - 1: 60 ticks (1/8 beat)
/// - 2: 120 ticks (1/4 beat, sixteenth note)
/// - 3: 240 ticks (1/2 beat, eighth note)
/// - 4: 480 ticks (1 beat, quarter note)
/// - 5: 960 ticks (2 beats, half note)
pub fn ticks_per_cell(zoom_level: u8) -> u32 {
    match zoom_level {
        1 => 60,
        2 => 120,
        3 => 240,
        4 => 480,
        5 => 960,
        _ => 240,
    }
}

/// Snap a tick position to the nearest grid boundary.
pub fn snap_to_grid(tick: u32, zoom_level: u8) -> u32 {
    let grid = ticks_per_cell(zoom_level);
    (tick / grid) * grid
}

/// Normalize a tick range from anchor/cursor into (start, end) with start <= end.
pub fn normalize_tick_range(anchor: u32, cursor: u32) -> (u32, u32) {
    if anchor <= cursor {
        (anchor, cursor)
    } else {
        (cursor, anchor)
    }
}

/// Normalize a 2D selection region (ticks x pitch) from anchor/cursor positions.
/// Returns (start_tick, end_tick, low_pitch, high_pitch).
/// `cell_width` is added to the end tick to make the selection inclusive of the last cell.
pub fn normalize_2d_region(
    anchor_tick: u32,
    anchor_pitch: u8,
    cursor_tick: u32,
    cursor_pitch: u8,
    cell_width: u32,
) -> (u32, u32, u8, u8) {
    let (t0, t1) = if anchor_tick <= cursor_tick {
        (anchor_tick, cursor_tick + cell_width)
    } else {
        (cursor_tick, anchor_tick + cell_width)
    };
    let (p0, p1) = if anchor_pitch <= cursor_pitch {
        (anchor_pitch, cursor_pitch)
    } else {
        (cursor_pitch, anchor_pitch)
    };
    (t0, t1, p0, p1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_per_cell_all_levels() {
        assert_eq!(ticks_per_cell(1), 60);
        assert_eq!(ticks_per_cell(2), 120);
        assert_eq!(ticks_per_cell(3), 240);
        assert_eq!(ticks_per_cell(4), 480);
        assert_eq!(ticks_per_cell(5), 960);
    }

    #[test]
    fn ticks_per_cell_default() {
        assert_eq!(ticks_per_cell(0), 240);
        assert_eq!(ticks_per_cell(6), 240);
        assert_eq!(ticks_per_cell(255), 240);
    }

    #[test]
    fn snap_to_grid_aligned() {
        assert_eq!(snap_to_grid(480, 4), 480);
        assert_eq!(snap_to_grid(0, 3), 0);
    }

    #[test]
    fn snap_to_grid_rounds_down() {
        assert_eq!(snap_to_grid(500, 4), 480);
        assert_eq!(snap_to_grid(959, 4), 480);
        assert_eq!(snap_to_grid(100, 3), 0);
        assert_eq!(snap_to_grid(250, 3), 240);
    }

    #[test]
    fn normalize_tick_range_ordered() {
        assert_eq!(normalize_tick_range(100, 500), (100, 500));
        assert_eq!(normalize_tick_range(500, 100), (100, 500));
        assert_eq!(normalize_tick_range(200, 200), (200, 200));
    }

    #[test]
    fn normalize_2d_region_anchor_before_cursor() {
        let (t0, t1, p0, p1) = normalize_2d_region(0, 60, 480, 72, 240);
        assert_eq!(t0, 0);
        assert_eq!(t1, 720); // 480 + 240
        assert_eq!(p0, 60);
        assert_eq!(p1, 72);
    }

    #[test]
    fn normalize_2d_region_cursor_before_anchor() {
        let (t0, t1, p0, p1) = normalize_2d_region(480, 72, 0, 60, 240);
        assert_eq!(t0, 0);
        assert_eq!(t1, 720); // 480 + 240
        assert_eq!(p0, 60);
        assert_eq!(p1, 72);
    }

    #[test]
    fn normalize_2d_region_same_position() {
        let (t0, t1, p0, p1) = normalize_2d_region(480, 60, 480, 60, 240);
        assert_eq!(t0, 480);
        assert_eq!(t1, 720);
        assert_eq!(p0, 60);
        assert_eq!(p1, 60);
    }

    #[test]
    fn snap_to_grid_zero() {
        assert_eq!(snap_to_grid(0, 1), 0);
        assert_eq!(snap_to_grid(0, 3), 0);
        assert_eq!(snap_to_grid(0, 5), 0);
    }

    #[test]
    fn snap_to_grid_exact_boundary() {
        // Already on grid boundary should stay unchanged
        assert_eq!(snap_to_grid(240, 3), 240);
        assert_eq!(snap_to_grid(960, 5), 960);
        assert_eq!(snap_to_grid(120, 2), 120);
    }
}
