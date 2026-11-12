//! Exact sums of floats and ints as big integers, and correctly rounded conversion back to f64.
//! Mirrors CPython's statistics._sum / _ss / _convert / _float_sqrt_of_frac for float and int data.
use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::BTreeMap;

const SX_OFF: i64 = 1074; // value m*2^e  -> index e+1074
const SXX_OFF: i64 = 2148; // m^2*2^(2e)  -> index 2e+2148

/// Sum of v*2^(idx-offset) with i128 buckets that spill into big integers on overflow.
struct ExpAcc {
    small: Vec<i128>,
    big: BTreeMap<usize, BigInt>,
}

impl ExpAcc {
    fn new(n: usize) -> Self {
        ExpAcc { small: vec![0; n], big: BTreeMap::new() }
    }
    #[inline]
    fn add(&mut self, idx: usize, v: i128) {
        match self.small[idx].checked_add(v) {
            Some(s) => self.small[idx] = s,
            None => {
                *self.big.entry(idx).or_default() += BigInt::from(self.small[idx]);
                self.small[idx] = v;
            }
        }
    }
    /// Σ bucket[idx] << idx  as an exact big integer.
    fn total(&self) -> BigInt {
        let mut t = BigInt::zero();
        for (i, &v) in self.small.iter().enumerate() {
            if v != 0 {
                t += BigInt::from(v) << i;
            }
        }
        for (&i, v) in &self.big {
            t += v << i;
        }
        t
    }
}

/// Decompose a finite f64 into (m, e) with value = m * 2^e, |m| < 2^53, e in [-1074, 971].
#[inline]
fn decompose(x: f64) -> (i64, i64) {
    let bits = x.to_bits();
    let sign = if bits >> 63 == 1 { -1 } else { 1 };
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let frac = (bits & ((1u64 << 52) - 1)) as i64;
    if exp == 0 {
        (sign * frac, -1074)
    } else {
        (sign * (frac | (1i64 << 52)), exp - 1075)
    }
}

pub enum Num {
    F(f64),
    I(i64),
}

pub struct Sums {
    pub n: u64,
    pub any_float: bool,
    /// running float sum of the inf/nan values, in input order (statistics: partials[None])
    pub special: Option<f64>,
    /// exact Σx = a / 2^1074
    pub a: BigInt,
    /// exact Σx² = b / 2^2148 (only when squares were requested)
    pub b: BigInt,
}

pub fn sums(data: &[Num], squares: bool) -> Sums {
    let mut sx = ExpAcc::new(2 * SX_OFF as usize + 2);
    let mut sxx = ExpAcc::new(if squares { 2 * SXX_OFF as usize + 4 } else { 0 });
    let mut special: Option<f64> = None;
    let mut any_float = false;
    for x in data {
        match *x {
            Num::F(f) => {
                any_float = true;
                if !f.is_finite() {
                    special = Some(match special { None => 0.0 + f, Some(s) => s + f });
                    continue;
                }
                let (m, e) = decompose(f);
                sx.add((e + SX_OFF) as usize, m as i128);
                if squares {
                    sxx.add((2 * e + SXX_OFF) as usize, (m as i128) * (m as i128));
                }
            }
            Num::I(i) => {
                sx.add(SX_OFF as usize, i as i128);
                if squares {
                    sxx.add(SXX_OFF as usize, (i as i128) * (i as i128));
                }
            }
        }
    }
    Sums {
        n: data.len() as u64,
        any_float,
        special,
        a: sx.total(),
        b: if squares { sxx.total() } else { BigInt::zero() },
    }
}

#[derive(Debug)]
pub struct Overflow;

/// Correctly rounded (round-half-even) num/den as f64, like int/int true division in CPython.
pub fn ratio_to_f64(num: &BigInt, den: &BigUint) -> Result<f64, Overflow> {
    if num.is_zero() {
        return Ok(0.0);
    }
    let neg = num.sign() == Sign::Minus;
    let a = num.magnitude();
    let (la, lb) = (a.bits() as i64, den.bits() as i64);
    let k = 55 - (la - lb);
    let (q, r) = if k >= 0 { (a << (k as u64)).div_rem(den) } else { a.div_rem(&(den << ((-k) as u64))) };
    let qb = q.bits() as i64;
    let e = qb - 1 - k; // floor(log2 |value|)
    let keep = if e >= -1022 { 53 } else { 53 - (-1022 - e) };
    let drop = qb - keep; // > 0
    let mut m = &q >> (drop as u64);
    let rem = &q - (&m << (drop as u64));
    let half = BigUint::one() << ((drop - 1) as u64);
    let up = rem > half || (rem == half && (!r.is_zero() || m.is_odd()));
    if up {
        m += 1u32;
    }
    let mut m = m.to_u64().unwrap();
    let bits = if e >= -1022 {
        let mut ee = e;
        if m == 1u64 << 53 {
            m >>= 1;
            ee += 1;
        }
        if ee > 1023 {
            return Err(Overflow);
        }
        (((ee + 1023) as u64) << 52) | (m - (1u64 << 52))
    } else {
        m // subnormal (m == 2^52 becomes the smallest normal, which is the same bit pattern)
    };
    let x = f64::from_bits(bits);
    Ok(if neg { -x } else { x })
}

/// Result of statistics._convert(value, T) for value = num/den.
pub enum Out {
    Int(BigInt),
    Float(f64),
}

pub fn convert(num: &BigInt, den: &BigUint, int_type: bool) -> Result<Out, Overflow> {
    if int_type {
        let d = BigInt::from_biguint(Sign::Plus, den.clone());
        let (q, r) = num.div_rem(&d);
        if r.is_zero() {
            return Ok(Out::Int(q));
        }
    }
    Ok(Out::Float(ratio_to_f64(num, den)?))
}

/// (numerator, denominator) of Σ(x - mean)² / divisor, unreduced, exact.
/// ss = (n·Σx² − (Σx)²)/n  with Σx = a/2^1074, Σx² = b/2^2148  ->  (n·b − a²) / (n · 2^2148)
pub fn ss_over(s: &Sums, divisor: u64) -> (BigInt, BigUint) {
    let n = BigInt::from(s.n);
    let num = &n * &s.b - &s.a * &s.a;
    let den = BigUint::from(s.n) * BigUint::from(divisor) << (SXX_OFF as u64);
    (num, den)
}

pub fn mean_frac(s: &Sums) -> (BigInt, BigUint) {
    (s.a.clone(), BigUint::from(s.n) << (SX_OFF as u64))
}

/// statistics._float_sqrt_of_frac on the reduced fraction num/den (num >= 0).
pub fn sqrt_of_frac(num: &BigInt, den: &BigUint) -> Result<f64, Overflow> {
    let d = BigInt::from_biguint(Sign::Plus, den.clone());
    let g = num.gcd(&d);
    let (n, m) = if g.is_zero() { (num.clone(), d) } else { (num / &g, &d / &g) };
    let n = n.abs().to_biguint().unwrap();
    let m = m.to_biguint().unwrap();
    let rto = |n: &BigUint, m: &BigUint| -> BigUint {
        let a = (n / m).sqrt();
        let exact = &a * &a * m == *n;
        if exact { a } else { a | BigUint::one() }
    };
    let width = 2 * 53 + 3;
    let q = Integer::div_floor(&(n.bits() as i64 - m.bits() as i64 - width), &2);
    let (numer, denom) = if q >= 0 {
        (rto(&n, &(&m << (2 * q as u64))) << (q as u64), BigUint::one())
    } else {
        (rto(&(&n << ((-2 * q) as u64)), &m), BigUint::one() << ((-q) as u64))
    };
    // like CPython: numerator / denominator may overflow for data near the float limits
    ratio_to_f64(&BigInt::from_biguint(Sign::Plus, numer), &denom)
}
