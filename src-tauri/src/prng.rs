// xorshift64* PRNG (multiplier 0x2545F4914F6CDD1D), FNV-1a seeding, and the
// three draw shapes from spec §7.1 (symmetric ±amp, [lo,hi] range, integer range).

const XORSHIFT_MUL: u64 = 0x2545F4914F6CDD1D;

pub struct Prng {
    state: u64,
}

impl Prng {
    pub fn from_seed(seed: u64) -> Self {
        // must be non-zero for xorshift
        Self { state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed } }
    }

    // FNV-1a of the output name → base seed
    pub fn fnv1a(bytes: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    // per-output stir: seed = base + idx*0x26f5, then | 1
    pub fn per_output(base: u64, idx: u64) -> Self {
        let seed = base.wrapping_add(idx.wrapping_mul(0x26f5)) | 1;
        Self::from_seed(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(XORSHIFT_MUL)
    }

    // u in [0,1) via top 53 bits ÷ 2^53
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    // symmetric ±: amp*(2u-1)
    pub fn symmetric(&mut self, amp: f64) -> f64 {
        amp * (2.0 * self.next_f64() - 1.0)
    }

    // range [lo,hi]: lo + u*(hi-lo)
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.next_f64() * (hi - lo)
    }

    // integer range: lo + rand % (hi-lo+1)
    pub fn int_range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }
}
