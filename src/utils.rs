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