use flip_engine::TICKS;

pub const GRIDS: [(&str, u32); 4] = [
    ("OFF", 1),
    ("1/32", TICKS / 2),
    ("1/16", TICKS),
    ("1/8", TICKS * 2),
];

pub fn grid(index: usize) -> u32 {
    GRIDS[index.min(GRIDS.len() - 1)].1
}

pub fn snap(tick: f64, grid: u32, total: u32) -> u32 {
    let total = total.max(1);
    let snapped = if grid > 1 {
        (tick / grid as f64).round() * grid as f64
    } else {
        tick.floor()
    };
    (snapped.max(0.0) as u32) % total
}

pub fn close(start: f64, end: f64, grid: u32, total: u32) -> (u32, u32) {
    let total = total.max(1);
    let mut length = end - start;
    if length <= 0.0 {
        length += total as f64;
    }
    let length = (length.round() as u32).clamp(1, total);
    (snap(start, grid, total), length)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOTAL: u32 = 16 * TICKS;

    mod when_closing_a_note {
        use super::*;

        #[test]
        fn keeps_the_held_length() {
            assert_eq!(close(30.0, 80.0, 1, TOTAL), (30, 50));
        }

        #[test]
        fn snaps_the_start_to_the_grid() {
            assert_eq!(close(30.0, 80.0, TICKS, TOTAL).0, 24);
            assert_eq!(close(40.0, 80.0, TICKS, TOTAL).0, 48);
        }

        #[test]
        fn wraps_across_the_loop_end() {
            let start = TOTAL as f64 - 12.0;
            assert_eq!(close(start, 12.0, 1, TOTAL), (TOTAL - 12, 24));
        }

        #[test]
        fn wraps_a_start_snapped_past_the_end() {
            assert_eq!(close(TOTAL as f64 - 2.0, 30.0, TICKS, TOTAL).0, 0);
        }
    }
}
