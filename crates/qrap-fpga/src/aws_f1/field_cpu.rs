//! Reference CPU implementation of 256-bit BN254 field arithmetic
use super::{Fe256, ProjPoint, AffinePoint};

/// BN254 prime p (little-endian)
pub const P: Fe256 = Fe256 {
    d: [
        0x3c208c16d87cfd47,
        0x97816a916871ca8d,
        0xb85045b68181585d,
        0x30644e72e131a029,
    ],
};

/// Montgomery constant NP = -p^{-1} mod 2^64
const NP: u64 = 0x87d20782e4866389;

impl Fe256 {
    /// a + b, returns (sum, carry)
    pub fn add_raw(a: &Fe256, b: &Fe256) -> (Fe256, bool) {
        let mut c = Fe256::default();
        let (s0, carry) = a.d[0].overflowing_add(b.d[0]);
        c.d[0] = s0;
        let (s1, carry) = a.d[1].overflowing_add(b.d[1] + carry as u64);
        c.d[1] = s1;
        let (s2, carry) = a.d[2].overflowing_add(b.d[2] + carry as u64);
        c.d[2] = s2;
        let (s3, carry) = a.d[3].overflowing_add(b.d[3] + carry as u64);
        c.d[3] = s3;
        (c, carry)
    }

    /// a - b, returns (diff, borrow)
    pub fn sub_raw(a: &Fe256, b: &Fe256) -> (Fe256, bool) {
        let mut c = Fe256::default();
        let (s0, borrow) = a.d[0].overflowing_sub(b.d[0]);
        c.d[0] = s0;
        let t1 = b.d[1] + borrow as u64;
        let (s1, borrow) = a.d[1].overflowing_sub(t1);
        c.d[1] = s1;
        let t2 = b.d[2] + borrow as u64;
        let (s2, borrow) = a.d[2].overflowing_sub(t2);
        c.d[2] = s2;
        let t3 = b.d[3] + borrow as u64;
        let (s3, borrow) = a.d[3].overflowing_sub(t3);
        c.d[3] = s3;
        (c, borrow)
    }

    /// a + b mod p
    pub fn add_mod(a: &Fe256, b: &Fe256) -> Fe256 {
        let (mut c, carry) = Self::add_raw(a, b);
        if carry {
            let (c2, _) = Self::sub_raw(&c, &P);
            c = c2;
        } else {
            let (_, borrow) = Self::sub_raw(&c, &P);
            if !borrow {
                let (c2, _) = Self::sub_raw(&c, &P);
                c = c2;
            }
        }
        c
    }

    /// a - b mod p
    pub fn sub_mod(a: &Fe256, b: &Fe256) -> Fe256 {
        let (c, borrow) = Self::sub_raw(a, b);
        if borrow {
            Self::add_raw(&c, &P).0
        } else {
            c
        }
    }

    /// 128-bit multiply
    fn mul128(a: u64, b: u64) -> (u64, u64) {
        let prod = (a as u128) * (b as u128);
        ((prod >> 64) as u64, prod as u64)
    }

    /// 256×256 → 512 bit multiply
    pub fn mul_raw(a: &Fe256, b: &Fe256) -> [u64; 8] {
        let mut t = [0u64; 8];
        for i in 0..4 {
            let mut carry = 0u64;
            for j in 0..4 {
                let (hi, lo) = Self::mul128(a.d[i], b.d[j]);
                let (s0, c0) = t[i + j].overflowing_add(lo);
                t[i + j] = s0;
                let (s1, c1) = s0.overflowing_add(carry);
                t[i + j] = s1;
                carry = hi + c0 as u64 + c1 as u64;
            }
            t[i + 4] = t[i + 4].wrapping_add(carry);
        }
        t
    }

    /// Montgomery multiplication: a * b * R^{-1} mod p
    pub fn mont_mul(a: &Fe256, b: &Fe256) -> Fe256 {
        let mut t = Self::mul_raw(a, b);

        for i in 0..4 {
            let m = t[i].wrapping_mul(NP);
            let mut carry = 0u64;
            for j in 0..4 {
                let (hi, lo) = Self::mul128(m, P.d[j]);
                let (s0, c0) = t[i + j].overflowing_add(lo);
                let (s1, c1) = s0.overflowing_add(carry);
                t[i + j] = s1;
                carry = hi + c0 as u64 + c1 as u64;
            }
            let (s2, c2) = t[i + 4].overflowing_add(carry);
            t[i + 4] = s2;
            if i + 5 < 8 {
                t[i + 5] = t[i + 5].wrapping_add(c2 as u64);
            }
        }

        let mut res = Fe256 {
            d: [t[4], t[5], t[6], t[7]],
        };
        let (_, borrow) = Self::sub_raw(&res, &P);
        if !borrow {
            let (r2, _) = Self::sub_raw(&res, &P);
            res = r2;
        }
        res
    }
}

/// EC addition stub: Jacobian + Affine -> Jacobian
pub fn ec_add_mixed(r: &mut ProjPoint, p: &AffinePoint) {
    if r.z.is_zero() {
        r.x = p.x;
        r.y = p.y;
        r.z = Fe256 { d: [1, 0, 0, 0] };
    }
}
