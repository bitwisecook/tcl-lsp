//! A tiny deterministic PRNG (`xorshift64*`) so a campaign is fully
//! reproducible from a seed without pulling in the `rand` crate.

/// A reproducible `xorshift64*` generator. Identical seeds yield identical
/// streams, so any finding can be replayed from its recorded seed.
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Seed the generator. A zero seed is remapped (xorshift requires a
    /// non-zero state).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    /// Next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform-ish integer in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        usize::try_from(self.next_u64() % n as u64).unwrap_or(0)
    }

    /// `true` with probability `num/den`.
    pub fn chance(&mut self, num: u32, den: u32) -> bool {
        debug_assert!(den > 0);
        (self.next_u64() % u64::from(den)) < u64::from(num)
    }

    /// Pick a reference to a random element of a non-empty slice.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}
