// Simple linear congruential generator for better randomness
// The `pub` keyword makes this struct visible to other modules.
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    /// Creates a new SimpleRng instance with a seed derived from the current system time.
    /// The `pub` keyword makes this constructor visible to other modules.
    pub fn new() -> Self {
        Self {
            state: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        }
    }

    /// Generates the next pseudo-random u64 number using a linear congruential generator.
    /// The `pub` keyword makes this method visible to other modules.
    pub fn next(&mut self) -> u64 {
        // LCG formula: X_n+1 = (a * X_n + c) mod m
        // Using constants from Numerical Recipes in C for a good LCG.
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }

    /// Generates the next pseudo-random f32 number between 0.0 and 1.0 (exclusive).
    /// The `pub` keyword makes this method visible to other modules.
    pub fn next_f32(&mut self) -> f32 {
        // Generate a u64 number, then mask it to 24 bits (0xFFFFFF) for f32 precision,
        // and normalize it to the range [0.0, 1.0).
        (self.next() & 0xFFFFFF) as f32 / 0xFFFFFF as f32
    }
}

// Configuration variables for the parrot pet
pub const ALWAYS_ON_TOP: bool = true;
pub const COLOR: windows::Win32::Foundation::COLORREF = windows::Win32::Foundation::COLORREF(0); // transparent color for the background

// Bubble and text configuration
pub const PARROT_SCALE: u32 = 4; // Parrot image scale divisor
pub const BUBBLE_SCALE: u32 = 4; // Bubble image scale divisor
pub const BUBBLE_OFFSET_X: i32 = 40; // Bubble X offset relative to parrot
pub const BUBBLE_OFFSET_Y: i32 = -50; // Bubble Y offset relative to parrot
pub const BUBBLE_TEXT_START_X: i32 = 80; // Text start X inside bubble
pub const BUBBLE_TEXT_START_Y: i32 = 120; // Text start Y inside bubble
pub const FIRST_LINE_SPACING: i32 = 32; // Space after first line in bubble
pub const OTHER_LINE_SPACING: i32 = 20; // Space after other lines in bubble
pub const FONT_SIZE_HEAD: f32 = 25.0; // Font size for first two chars
pub const FONT_SIZE_MAIN: f32 = 18.0; // Font size for rest of text
pub const REMINDER_INTERVAL: u64 = 1800; // How often to queue a reminder (in seconds) 30min/1800