//! Deterministic PRNG (SplitMix64 + Xoshiro256**) with `ffh_battle::BattleRng` support.

use ffh_battle::BattleRng;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    state: [u64; 4],
}

fn splitmix64(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut s = seed ^ 0x517c_c1b7_2722_0a95;
        let state = [splitmix64(&mut s), splitmix64(&mut s), splitmix64(&mut s), splitmix64(&mut s)];
        Self { state }
    }

    pub fn from_state(state: [u64; 4]) -> Self {
        Self { state }
    }

    pub fn state(&self) -> [u64; 4] {
        self.state
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let result = self.state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 17;
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);
        result
    }

    /// Uniform integer in `0..bound` (0 if `bound <= 0`).
    #[inline]
    pub fn range(&mut self, bound: i32) -> i32 {
        if bound <= 1 {
            return 0;
        }
        (self.next_u64() % (bound as u64)) as i32
    }

    #[inline]
    pub fn range_usize(&mut self, bound: usize) -> usize {
        if bound <= 1 {
            return 0;
        }
        (self.next_u64() % (bound as u64)) as usize
    }

    /// Returns `true` with `pct`% probability (`0..=100`).
    #[inline]
    pub fn chance(&mut self, pct: i32) -> bool {
        if pct <= 0 {
            return false;
        }
        if pct >= 100 {
            return true;
        }
        self.range(100) < pct
    }

    /// Deterministic Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.range_usize(i + 1);
            slice.swap(i, j);
        }
    }
}

impl BattleRng for Rng {
    fn rand(&mut self, n: i32, _label: &'static str) -> i32 {
        self.range(n)
    }
}
