use std::hash::Hasher;

pub struct DotnetRng {
    state: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl DotnetRng {
    pub fn new(seed: i32) -> Self {
        // Reference:
        // https://github.com/dotnet/runtime/blob/a45853c4751b5f532cdb38e3db5d4324b5ca878a/src/libraries/System.Private.CoreLib/src/System/Random.Net5CompatImpl.cs#L258
        let mut state = [0_i32; 56];
        let mut mj = 161803398 - seed.saturating_abs();
        state[55] = mj;
        let mut mk = 1_i32;
        let mut ii = 0;
        for _i in 1..55 {
            // this should be a 31 instead lmao
            ii = (ii + 21) % 55;
            state[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += i32::MAX;
            }
            mj = state[ii];
        }
        for _k in 1..5 {
            for i in 1..56 {
                let n = (i + 30) % 55;
                state[i] = state[i].wrapping_sub(state[1 + n]);
                if state[i] < 0 {
                    state[i] += i32::MAX;
                };
            }
        }
        DotnetRng {
            state,
            inextp: 21,
            inext: 0,
        }
    }
    pub fn next(&mut self) -> i32 {
        self.inext = (self.inext % 55) + 1;
        self.inextp = (self.inextp % 55) + 1;
        let mut result = self.state[self.inext].wrapping_sub(self.state[self.inextp]);
        // the dotnet random api is Very High Quality
        // then again, which part of this implementation isn't?
        if result == i32::MAX {
            result -= 1;
        }
        if result < 0 {
            result += i32::MAX;
        }
        // also extremely funny how they write this, instead of the unmangled result,
        // back into the state array
        self.state[self.inext] = result;
        result
    }
    pub fn next_f64(&mut self) -> f64 {
        self.next() as f64 * (1.0 / i32::MAX as f64)
    }
    pub fn next_range(&mut self, max: i32) -> i32 {
        (self.next_f64() * max as f64) as i32
    }
}

fn stardew_hashcode(data: &[u8]) -> i32 {
    let mut hasher = twox_hash::XxHash32::with_seed(0);
    hasher.write(data);
    // XxHash32's finish() always returns a valid u32. stardew casts this to i32 assuming 2's
    // complement. rust's `as` also assumes 2's complement.
    (hasher.finish() as u32) as i32
}

pub fn stardew_seed_mix(legacy_rng: bool, values: &[f64]) -> i32 {
    debug_assert!(values.len() <= 5);
    if legacy_rng {
        // this is technically not perfectly accurate to 1.5 i think?? but maybe it's close enough
        // for anything that matters
        values.iter().map(|x| x % 2147483647.0).sum::<f64>() as i32
    } else {
        let vals: Vec<i32> = values.iter().map(|x| (x % 2147483647.0) as i32).collect();
        let mut h = [0_i32; 5];
        h[0..vals.len()].copy_from_slice(&vals);
        let bytes: Vec<u8> = h.iter().flat_map(|x| x.to_le_bytes()).collect();
        stardew_hashcode(&bytes)
    }
}
