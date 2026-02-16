// SPDX-License-Identifier: AGPL-3.0

//! Symbolic bit vector operations for EVM execution
//!
//! This module provides wrappers around Z3 bit vectors with additional
//! functionality for symbolic execution of EVM bytecode.

use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{One, ToPrimitive, Zero};
use std::fmt;
use z3::ast::{Ast, Bool as Z3Bool, BV};
use z3::{Context, FuncDecl};

use cbse_exceptions::{CbseException, CbseResult};

fn mask(bit_size: u32) -> BigUint {
    if bit_size == 0 {
        BigUint::zero()
    } else {
        (BigUint::one() << bit_size as usize) - BigUint::one()
    }
}

fn normalize_biguint(value: BigUint, bit_size: u32) -> BigUint {
    if bit_size == 0 {
        BigUint::zero()
    } else {
        value & mask(bit_size)
    }
}

fn biguint_is_power_of_two(value: &BigUint) -> bool {
    if value.is_zero() {
        return false;
    }

    let mut minus_one = value.clone();
    minus_one -= BigUint::one();
    (value & &minus_one).is_zero()
}

fn biguint_to_bv<'ctx>(ctx: &'ctx Context, value: &BigUint, bit_size: u32) -> BV<'ctx> {
    if bit_size == 0 {
        panic!("Bit size must be greater than zero");
    }

    if value.is_zero() {
        return BV::from_u64(ctx, 0, bit_size);
    }

    if let Some(small) = value.to_u64() {
        return BV::from_u64(ctx, small, bit_size);
    }

    // Build the bitvector byte-by-byte to support arbitrary sizes
    let num_bytes = ((bit_size as usize) + 7) / 8;
    let mut bytes = value.to_bytes_be();
    if bytes.len() < num_bytes {
        let mut padded = vec![0u8; num_bytes - bytes.len()];
        padded.extend_from_slice(&bytes);
        bytes = padded;
    } else if bytes.len() > num_bytes {
        bytes = bytes[bytes.len() - num_bytes..].to_vec();
    }

    if bit_size <= 8 {
        let value = bytes.last().copied().unwrap_or(0) as u64;
        return BV::from_u64(ctx, value, bit_size);
    }

    let mut iter = bytes.into_iter();
    let first = iter.next().unwrap_or(0);
    let mut acc = BV::from_u64(ctx, first as u64, 8);
    for byte in iter {
        let next = BV::from_u64(ctx, byte as u64, 8);
        acc = acc.concat(&next);
    }

    if bit_size % 8 == 0 {
        acc
    } else {
        acc.extract(bit_size - 1, 0)
    }
}

fn apply_func_decl<'ctx>(decl: &FuncDecl<'ctx>, args: &[BV<'ctx>]) -> BV<'ctx> {
    let ast_args: Vec<&dyn Ast<'ctx>> = args.iter().map(|arg| arg as &dyn Ast<'ctx>).collect();
    decl.apply(&ast_args)
        .as_bv()
        .expect("Function declaration must return a bit-vector")
}

fn to_signed_bigint(value: &BigUint, bit_size: u32) -> BigInt {
    if bit_size == 0 {
        return BigInt::zero();
    }

    let bit_size_usize = bit_size as usize;
    let sign_bit = BigUint::one() << (bit_size_usize - 1);
    if value < &sign_bit {
        BigInt::from(value.clone())
    } else {
        let modulus = BigUint::one() << bit_size_usize;
        BigInt::from_biguint(Sign::Minus, modulus - value)
    }
}

fn bigint_to_twos_complement(value: &BigInt, bit_size: u32) -> BigUint {
    if bit_size == 0 {
        return BigUint::zero();
    }

    let modulus = BigUint::one() << bit_size as usize;

    match value.sign() {
        Sign::NoSign => BigUint::zero(),
        Sign::Plus => normalize_biguint(value.to_biguint().unwrap(), bit_size),
        Sign::Minus => {
            let magnitude = (-value.clone()).to_biguint().unwrap();
            if magnitude.is_zero() {
                BigUint::zero()
            } else {
                normalize_biguint(modulus - magnitude, bit_size)
            }
        }
    }
}

fn biguint_from_bytes(bytes: &[u8]) -> BigUint {
    if bytes.is_empty() {
        BigUint::zero()
    } else {
        BigUint::from_bytes_be(bytes)
    }
}

/// Check if a number is a power of two
#[inline]
pub fn is_power_of_two(x: u64) -> bool {
    x > 0 && (x & (x - 1)) == 0
}

/// Convert unsigned integer to signed representation
pub fn to_signed(x: u64, bit_size: usize) -> i64 {
    let sign_bit = 1u64 << (bit_size - 1);
    if x & sign_bit != 0 {
        x.wrapping_sub(1 << bit_size) as i64
    } else {
        x as i64
    }
}

/// Symbolic or concrete boolean value
#[derive(Clone)]
pub enum CbseBool<'ctx> {
    Concrete(bool),
    Symbolic(Z3Bool<'ctx>),
}

impl<'ctx> CbseBool<'ctx> {
    /// Create a new concrete boolean
    pub fn from_bool(_ctx: &'ctx Context, value: bool) -> Self {
        Self::Concrete(value)
    }

    /// Create a new symbolic boolean
    pub fn from_z3(value: Z3Bool<'ctx>) -> Self {
        // Try to simplify to concrete if possible
        if let Some(simplified) = value.simplify().as_bool() {
            Self::Concrete(simplified)
        } else {
            Self::Symbolic(value)
        }
    }

    /// Check if this is concrete
    pub fn is_concrete(&self) -> bool {
        matches!(self, Self::Concrete(_))
    }

    /// Check if this is symbolic
    pub fn is_symbolic(&self) -> bool {
        matches!(self, Self::Symbolic(_))
    }

    /// Check whether this is the literal true
    pub fn is_true(&self) -> bool {
        matches!(self, Self::Concrete(true))
    }

    /// Check whether this is the literal false
    pub fn is_false(&self) -> bool {
        matches!(self, Self::Concrete(false))
    }

    /// Get concrete value, returns error if symbolic
    pub fn as_bool(&self) -> CbseResult<bool> {
        match self {
            Self::Concrete(b) => Ok(*b),
            Self::Symbolic(_) => Err(CbseException::NotConcrete(
                "Boolean is symbolic".to_string(),
            )),
        }
    }

    /// Structural equality between two boolean values
    pub fn eq(&self, other: &Self, ctx: &'ctx Context) -> Self {
        match (self, other) {
            (Self::Concrete(a), Self::Concrete(b)) => Self::Concrete(a == b),
            _ => {
                let lhs = self.as_z3(ctx);
                let rhs = other.as_z3(ctx);
                Self::from_z3(lhs._eq(&rhs))
            }
        }
    }

    /// Get as Z3 boolean
    pub fn as_z3(&self, ctx: &'ctx Context) -> Z3Bool<'ctx> {
        match self {
            Self::Concrete(b) => Z3Bool::from_bool(ctx, *b),
            Self::Symbolic(z3) => z3.clone(),
        }
    }

    /// Boolean negation
    pub fn neg(&self, ctx: &'ctx Context) -> Self {
        self.not(ctx)
    }

    /// Bitwise NOT (same as logical NOT for booleans)
    pub fn bitwise_not(&self, ctx: &'ctx Context) -> Self {
        self.not(ctx)
    }

    /// Logical AND
    pub fn and(&self, other: &Self, ctx: &'ctx Context) -> Self {
        match (self, other) {
            (Self::Concrete(false), _) | (_, Self::Concrete(false)) => Self::Concrete(false),
            (Self::Concrete(true), other) => other.clone(),
            (this, Self::Concrete(true)) => this.clone(),
            (Self::Symbolic(a), Self::Symbolic(b)) => Self::from_z3(Z3Bool::and(ctx, &[a, b])),
        }
    }

    /// Bitwise AND (alias for logical AND)
    pub fn bitwise_and(&self, other: &Self, ctx: &'ctx Context) -> Self {
        self.and(other, ctx)
    }

    /// Logical OR
    pub fn or(&self, other: &Self, ctx: &'ctx Context) -> Self {
        match (self, other) {
            (Self::Concrete(true), _) | (_, Self::Concrete(true)) => Self::Concrete(true),
            (Self::Concrete(false), other) => other.clone(),
            (this, Self::Concrete(false)) => this.clone(),
            (Self::Symbolic(a), Self::Symbolic(b)) => Self::from_z3(Z3Bool::or(ctx, &[a, b])),
        }
    }

    /// Bitwise OR (alias for logical OR)
    pub fn bitwise_or(&self, other: &Self, ctx: &'ctx Context) -> Self {
        self.or(other, ctx)
    }

    /// Logical NOT
    pub fn not(&self, _ctx: &'ctx Context) -> Self {
        match self {
            Self::Concrete(b) => Self::Concrete(!b),
            Self::Symbolic(z3) => Self::from_z3(z3.not()),
        }
    }

    /// Bitwise XOR
    pub fn bitwise_xor(&self, other: &Self, ctx: &'ctx Context) -> Self {
        if self.is_true() {
            return other.bitwise_not(ctx);
        }

        if other.is_true() {
            return self.bitwise_not(ctx);
        }

        if self.is_false() {
            return other.clone();
        }

        if other.is_false() {
            return self.clone();
        }

        match (self, other) {
            (Self::Symbolic(a), Self::Symbolic(b)) => Self::from_z3(a.iff(b).not()),
            _ => unreachable!("All boolean XOR cases should be covered above"),
        }
    }

    /// Whether this boolean represents zero (false)
    pub fn is_zero(&self, ctx: &'ctx Context) -> Self {
        self.not(ctx)
    }

    /// Whether this boolean represents a non-zero value (true)
    pub fn is_non_zero(&self) -> Self {
        self.clone()
    }

    /// Convert boolean to bitvec (0 or 1)
    pub fn to_bitvec(&self, ctx: &'ctx Context, size: u32) -> CbseBitVec<'ctx> {
        match self {
            Self::Concrete(true) => CbseBitVec::from_u64(1, size),
            Self::Concrete(false) => CbseBitVec::from_u64(0, size),
            Self::Symbolic(z3) => {
                let zero = BV::from_u64(ctx, 0, size);
                let one = BV::from_u64(ctx, 1, size);
                CbseBitVec::Symbolic {
                    value: z3.ite(&one, &zero),
                    size,
                }
            }
        }
    }

    /// Alias for [`to_bitvec`]
    pub fn as_bv(&self, ctx: &'ctx Context, size: u32) -> CbseBitVec<'ctx> {
        self.to_bitvec(ctx, size)
    }
}

impl<'ctx> fmt::Debug for CbseBool<'ctx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Concrete(b) => write!(f, "Bool({})", b),
            Self::Symbolic(z3) => write!(f, "Bool({})", z3),
        }
    }
}

/// Symbolic or concrete bit vector
#[derive(Clone)]
pub enum CbseBitVec<'ctx> {
    Concrete { value: BigUint, size: u32 },
    Symbolic { value: BV<'ctx>, size: u32 },
}

impl<'ctx> CbseBitVec<'ctx> {
    /// Create a concrete bit vector from u64
    pub fn from_u64(value: u64, size: u32) -> Self {
        Self::from_biguint(BigUint::from(value), size)
    }

    /// Create a concrete bit vector from BigUint
    pub fn from_biguint(value: BigUint, size: u32) -> Self {
        Self::Concrete {
            value: normalize_biguint(value, size),
            size,
        }
    }

    /// Create a concrete bit vector from bytes
    pub fn from_bytes(bytes: &[u8], size: u32) -> Self {
        Self::from_biguint(BigUint::from_bytes_be(bytes), size)
    }

    /// Create a concrete bit vector from a boolean value
    pub fn from_bool(value: bool, size: u32) -> Self {
        if value {
            Self::from_u64(1, size)
        } else {
            Self::from_u64(0, size)
        }
    }

    /// Create a symbolic bit vector
    pub fn from_z3(value: BV<'ctx>) -> Self {
        let size = value.get_size();
        Self::Symbolic { value, size }
    }

    /// Create a fresh symbolic variable
    pub fn symbolic(ctx: &'ctx Context, name: &str, size: u32) -> Self {
        Self::Symbolic {
            value: BV::new_const(ctx, name, size),
            size,
        }
    }

    /// Get the size in bits
    pub fn size(&self) -> u32 {
        match self {
            Self::Concrete { size, .. } => *size,
            Self::Symbolic { size, .. } => *size,
        }
    }

    /// Check if this is concrete
    pub fn is_concrete(&self) -> bool {
        matches!(self, Self::Concrete { .. })
    }

    /// Check if this is symbolic
    pub fn is_symbolic(&self) -> bool {
        matches!(self, Self::Symbolic { .. })
    }

    /// Get concrete value as u64, returns error if symbolic or too large
    pub fn as_u64(&self) -> CbseResult<u64> {
        match self {
            Self::Concrete { value, .. } => value
                .to_u64()
                .ok_or_else(|| CbseException::NotConcrete("Value too large for u64".to_string())),
            Self::Symbolic { .. } => {
                Err(CbseException::NotConcrete("BitVec is symbolic".to_string()))
            }
        }
    }

    /// Get concrete value as BigUint, returns error if symbolic
    pub fn as_biguint(&self) -> CbseResult<BigUint> {
        match self {
            Self::Concrete { value, .. } => Ok(value.clone()),
            Self::Symbolic { .. } => {
                Err(CbseException::NotConcrete("BitVec is symbolic".to_string()))
            }
        }
    }

    /// Get as Z3 bit vector
    pub fn as_z3(&self, ctx: &'ctx Context) -> BV<'ctx> {
        match self {
            Self::Concrete { value, size } => biguint_to_bv(ctx, value, *size),
            Self::Symbolic { value, .. } => value.clone(),
        }
    }

    /// Determine if the value is zero
    pub fn is_zero(&self, ctx: &'ctx Context) -> CbseBool<'ctx> {
        match self {
            Self::Concrete { value, .. } => CbseBool::Concrete(value.is_zero()),
            Self::Symbolic { value, size } => {
                let zero = BV::from_u64(ctx, 0, *size);
                CbseBool::from_z3(value._eq(&zero))
            }
        }
    }

    /// Determine if the value is non-zero
    pub fn is_non_zero(&self, ctx: &'ctx Context) -> CbseBool<'ctx> {
        self.is_zero(ctx).bitwise_not(ctx)
    }

    /// Addition
    pub fn add(&self, other: &Self, ctx: &'ctx Context) -> Self {
        assert_eq!(self.size(), other.size());
        match (self, other) {
            (Self::Concrete { value: a, size }, Self::Concrete { value: b, .. }) => {
                Self::from_biguint(a + b, *size)
            }
            _ => Self::from_z3(self.as_z3(ctx).bvadd(&other.as_z3(ctx))),
        }
    }

    /// Subtraction
    pub fn sub(&self, other: &Self, ctx: &'ctx Context) -> Self {
        assert_eq!(self.size(), other.size());
        match (self, other) {
            (Self::Concrete { value: a, size }, Self::Concrete { value: b, .. }) => {
                if a >= b {
                    Self::from_biguint(a - b, *size)
                } else {
                    let modulus = BigUint::one() << *size as usize;
                    let diff = b - a;
                    Self::from_biguint(modulus - diff, *size)
                }
            }
            _ => Self::from_z3(self.as_z3(ctx).bvsub(&other.as_z3(ctx))),
        }
    }

    /// Multiplication
    pub fn mul(&self, other: &Self, ctx: &'ctx Context) -> Self {
        self.mul_with_abstraction(other, ctx, None)
    }

    /// Multiplication with optional abstraction function
    pub fn mul_with_abstraction(
        &self,
        other: &Self,
        ctx: &'ctx Context,
        abstraction: Option<&FuncDecl<'ctx>>,
    ) -> Self {
        assert_eq!(self.size(), other.size());

        match (self, other) {
            (Self::Concrete { value: lhs, size }, Self::Concrete { value: rhs, .. }) => {
                Self::from_biguint(lhs * rhs, *size)
            }

            (Self::Concrete { value: lhs, size }, Self::Symbolic { value: rhs, .. }) => {
                if lhs.is_zero() {
                    return Self::from_u64(0, *size);
                }

                if lhs.is_one() {
                    return other.clone();
                }

                if biguint_is_power_of_two(lhs) {
                    let shift = lhs.bits().saturating_sub(1) as u64;
                    return other.shl(&Self::from_u64(shift, *size), ctx);
                }

                let lhs_bv = biguint_to_bv(ctx, lhs, *size);
                return Self::from_z3(rhs.bvmul(&lhs_bv));
            }

            (Self::Symbolic { value: lhs, size }, Self::Concrete { value: rhs, .. }) => {
                if rhs.is_zero() {
                    return Self::from_u64(0, *size);
                }

                if rhs.is_one() {
                    return self.clone();
                }

                if biguint_is_power_of_two(rhs) {
                    let shift = rhs.bits().saturating_sub(1) as u64;
                    return self.shl(&Self::from_u64(shift, *size), ctx);
                }

                let rhs_bv = biguint_to_bv(ctx, rhs, *size);
                return Self::from_z3(lhs.bvmul(&rhs_bv));
            }

            (Self::Symbolic { value: lhs, .. }, Self::Symbolic { value: rhs, .. }) => {
                let lhs_bv = lhs.clone();
                let rhs_bv = rhs.clone();

                if let Some(func) = abstraction {
                    return Self::from_z3(apply_func_decl(func, &[lhs_bv, rhs_bv]));
                }

                return Self::from_z3(lhs.bvmul(rhs));
            }
        }
    }

    /// Unsigned division
    pub fn udiv(&self, other: &Self, ctx: &'ctx Context) -> Self {
        self.udiv_with_abstraction(other, ctx, None)
    }

    /// Unsigned division with optional abstraction
    pub fn udiv_with_abstraction(
        &self,
        other: &Self,
        ctx: &'ctx Context,
        abstraction: Option<&FuncDecl<'ctx>>,
    ) -> Self {
        assert_eq!(self.size(), other.size());

        match (self, other) {
            (_, Self::Concrete { value, .. }) if value.is_zero() => Self::from_u64(0, self.size()),

            (_, Self::Concrete { value, .. }) if value.is_one() => self.clone(),

            (Self::Concrete { value: lhs, size }, Self::Concrete { value: rhs, .. }) => {
                if rhs.is_zero() {
                    Self::from_u64(0, *size)
                } else {
                    Self::from_biguint(lhs / rhs, *size)
                }
            }

            (Self::Symbolic { .. }, Self::Concrete { value: rhs, size })
                if biguint_is_power_of_two(rhs) =>
            {
                let shift = rhs.bits().saturating_sub(1) as u64;
                self.lshr(&Self::from_u64(shift, *size), ctx)
            }

            _ => {
                let lhs_bv = self.as_z3(ctx);
                let rhs_bv = other.as_z3(ctx);

                if let Some(func) = abstraction {
                    return Self::from_z3(apply_func_decl(func, &[lhs_bv.clone(), rhs_bv.clone()]));
                }

                Self::from_z3(lhs_bv.bvudiv(&rhs_bv))
            }
        }
    }

    /// Unsigned modulo
    pub fn urem(&self, other: &Self, ctx: &'ctx Context) -> Self {
        self.urem_with_abstraction(other, ctx, None)
    }

    /// Unsigned modulo with optional abstraction
    pub fn urem_with_abstraction(
        &self,
        other: &Self,
        ctx: &'ctx Context,
        abstraction: Option<&FuncDecl<'ctx>>,
    ) -> Self {
        assert_eq!(self.size(), other.size());

        match (self, other) {
            (_, Self::Concrete { value, .. }) if value.is_zero() => self.clone(),

            (Self::Concrete { value: lhs, size }, Self::Concrete { value: rhs, .. }) => {
                if rhs.is_zero() {
                    self.clone()
                } else {
                    Self::from_biguint(lhs % rhs, *size)
                }
            }

            (_, Self::Concrete { value: rhs, size }) if biguint_is_power_of_two(rhs) => {
                let bits = rhs.bits().saturating_sub(1) as u32;
                if bits == 0 {
                    return Self::from_u64(0, *size);
                }
                let truncated = self.truncate(bits, ctx);
                truncated.zero_extend(*size, ctx)
            }

            _ => {
                let lhs_bv = self.as_z3(ctx);
                let rhs_bv = other.as_z3(ctx);

                if let Some(func) = abstraction {
                    return Self::from_z3(apply_func_decl(func, &[lhs_bv.clone(), rhs_bv.clone()]));
                }

                Self::from_z3(lhs_bv.bvurem(&rhs_bv))
            }
        }
    }

    /// Signed division
    pub fn sdiv(&self, other: &Self, ctx: &'ctx Context) -> Self {
        self.sdiv_with_abstraction(other, ctx, None)
    }

    /// Signed division with optional abstraction
    pub fn sdiv_with_abstraction(
        &self,
        other: &Self,
        ctx: &'ctx Context,
        abstraction: Option<&FuncDecl<'ctx>>,
    ) -> Self {
        assert_eq!(self.size(), other.size());

        match (self, other) {
            (_, Self::Concrete { value, .. }) if value.is_zero() => Self::from_u64(0, self.size()),

            (_, Self::Concrete { value, .. }) if value.is_one() => self.clone(),

            (Self::Concrete { value: lhs, size }, Self::Concrete { value: rhs, .. }) => {
                if rhs.is_zero() {
                    return Self::from_u64(0, *size);
                }

                let lhs_signed = to_signed_bigint(lhs, *size);
                let rhs_signed = to_signed_bigint(rhs, *size);
                let result = lhs_signed / rhs_signed;
                Self::from_biguint(bigint_to_twos_complement(&result, *size), *size)
            }

            _ => {
                let lhs_bv = self.as_z3(ctx);
                let rhs_bv = other.as_z3(ctx);

                if let Some(func) = abstraction {
                    return Self::from_z3(apply_func_decl(func, &[lhs_bv.clone(), rhs_bv.clone()]));
                }

                Self::from_z3(lhs_bv.bvsdiv(&rhs_bv))
            }
        }
    }

    /// Signed modulo (remainder)
    pub fn smod(&self, other: &Self, ctx: &'ctx Context) -> Self {
        self.smod_with_abstraction(other, ctx, None)
    }

    /// Signed modulo with optional abstraction
    pub fn smod_with_abstraction(
        &self,
        other: &Self,
        ctx: &'ctx Context,
        abstraction: Option<&FuncDecl<'ctx>>,
    ) -> Self {
        assert_eq!(self.size(), other.size());

        match (self, other) {
            (_, Self::Concrete { value, .. }) if value.is_zero() => self.clone(),

            (Self::Concrete { value: lhs, size }, Self::Concrete { value: rhs, .. }) => {
                if rhs.is_zero() {
                    return self.clone();
                }

                let lhs_signed = to_signed_bigint(lhs, *size);
                let rhs_signed = to_signed_bigint(rhs, *size);
                let result = lhs_signed % rhs_signed;
                Self::from_biguint(bigint_to_twos_complement(&result, *size), *size)
            }

            _ => {
                let lhs_bv = self.as_z3(ctx);
                let rhs_bv = other.as_z3(ctx);

                if let Some(func) = abstraction {
                    return Self::from_z3(apply_func_decl(func, &[lhs_bv.clone(), rhs_bv.clone()]));
                }

                Self::from_z3(lhs_bv.bvsrem(&rhs_bv))
            }
        }
    }

    /// Exponentiation with optional abstractions
    #[allow(clippy::too_many_arguments)]
    pub fn exp(
        &self,
        other: &Self,
        ctx: &'ctx Context,
        exp_abstraction: Option<&FuncDecl<'ctx>>,
        mul_abstraction: Option<&FuncDecl<'ctx>>,
        smt_exp_by_const: u32,
    ) -> CbseResult<Self> {
        assert_eq!(self.size(), other.size());

        if other.is_concrete() {
            let exponent = other.as_biguint()?;

            if exponent.is_zero() {
                return Ok(Self::from_u64(1, self.size()));
            }

            if exponent.is_one() {
                return Ok(self.clone());
            }

            if self.is_concrete() {
                let base = self.as_biguint()?;
                let modulus = BigUint::one() << self.size() as usize;
                let result = base.modpow(&exponent, &modulus);
                return Ok(Self::from_biguint(result, self.size()));
            }

            if let Some(exp_u32) = exponent.to_u32() {
                if exp_u32 <= smt_exp_by_const {
                    if exp_u32 == 0 {
                        return Ok(Self::from_u64(1, self.size()));
                    }

                    let mut acc = self.clone();
                    for _ in 1..exp_u32 {
                        acc = acc.mul_with_abstraction(self, ctx, mul_abstraction);
                    }
                    return Ok(acc);
                }
            }
        }

        let abstraction = exp_abstraction.ok_or_else(|| {
            CbseException::Internal("Missing SMT abstraction for exponentiation".to_string())
        })?;

        let lhs_bv = self.as_z3(ctx);
        let rhs_bv = other.as_z3(ctx);
        Ok(Self::from_z3(apply_func_decl(
            abstraction,
            &[lhs_bv, rhs_bv],
        )))
    }

    /// Addition modulo a third operand
    pub fn addmod(
        &self,
        other: &Self,
        modulus: &Self,
        ctx: &'ctx Context,
        abstraction: Option<&FuncDecl<'ctx>>,
    ) -> Self {
        assert_eq!(self.size(), other.size());
        assert_eq!(self.size(), modulus.size());

        if self.is_concrete() && other.is_concrete() && modulus.is_concrete() {
            let a = self.as_biguint().unwrap();
            let b = other.as_biguint().unwrap();
            let n = modulus.as_biguint().unwrap();
            if n.is_zero() {
                return Self::from_u64(0, self.size());
            }
            return Self::from_biguint((a + b) % n, self.size());
        }

        let new_size = self.size() + 8;
        let sum = self
            .zero_extend(new_size, ctx)
            .add(&other.zero_extend(new_size, ctx), ctx);
        let modulus_ext = modulus.zero_extend(new_size, ctx);
        let reduced = sum.urem_with_abstraction(&modulus_ext, ctx, abstraction);
        reduced.truncate(self.size(), ctx)
    }

    /// Multiplication modulo a third operand
    pub fn mulmod(
        &self,
        other: &Self,
        modulus: &Self,
        ctx: &'ctx Context,
        mul_abstraction: Option<&FuncDecl<'ctx>>,
        mod_abstraction: Option<&FuncDecl<'ctx>>,
    ) -> Self {
        assert_eq!(self.size(), other.size());
        assert_eq!(self.size(), modulus.size());

        if self.is_concrete() && other.is_concrete() && modulus.is_concrete() {
            let a = self.as_biguint().unwrap();
            let b = other.as_biguint().unwrap();
            let n = modulus.as_biguint().unwrap();
            if n.is_zero() {
                return Self::from_u64(0, self.size());
            }
            return Self::from_biguint((a * b) % n, self.size());
        }

        let new_size = self.size() * 2;
        let product = self.zero_extend(new_size, ctx).mul_with_abstraction(
            &other.zero_extend(new_size, ctx),
            ctx,
            mul_abstraction,
        );
        let modulus_ext = modulus.zero_extend(new_size, ctx);
        let reduced = product.urem_with_abstraction(&modulus_ext, ctx, mod_abstraction);
        reduced.truncate(self.size(), ctx)
    }

    /// Sign-extend from the specified byte index (EVM semantics)
    pub fn signextend(&self, byte_index: u32, _ctx: &'ctx Context) -> Self {
        assert_eq!(self.size(), 256, "signextend expects a 256-bit value");

        if byte_index >= 31 {
            return self.clone();
        }

        let bits = (byte_index + 1) * 8;

        match self {
            Self::Concrete { value, .. } => {
                let truncated = normalize_biguint(value.clone(), bits);
                let sign_bit = BigUint::one() << (bits as usize - 1);
                if truncated >= sign_bit {
                    let extension_mask = mask(256) ^ mask(bits);
                    Self::from_biguint(truncated | extension_mask, 256)
                } else {
                    Self::from_biguint(truncated, 256)
                }
            }
            Self::Symbolic { value, .. } => {
                let low = value.extract(bits - 1, 0);
                let extended = low.sign_ext(256 - bits);
                Self::from_z3(extended)
            }
        }
    }

    /// Zero-extend this bitvector to a larger size
    pub fn zero_extend(&self, new_size: u32, _ctx: &'ctx Context) -> Self {
        assert!(
            new_size >= self.size(),
            "can only zero-extend to a larger size"
        );
        if new_size == self.size() {
            return self.clone();
        }

        match self {
            Self::Concrete { value, .. } => Self::from_biguint(value.clone(), new_size),
            Self::Symbolic { value, size } => {
                let extra = new_size - size;
                Self::from_z3(value.zero_ext(extra))
            }
        }
    }

    /// Truncate this bitvector to a smaller size
    pub fn truncate(&self, new_size: u32, _ctx: &'ctx Context) -> Self {
        assert!(
            new_size <= self.size(),
            "can only truncate to a smaller size"
        );
        if new_size == self.size() {
            return self.clone();
        }

        match self {
            Self::Concrete { value, .. } => Self::from_biguint(value.clone(), new_size),
            Self::Symbolic { value, .. } => {
                let high = new_size.saturating_sub(1);
                Self::from_z3(value.extract(high, 0))
            }
        }
    }

    /// Concatenate two bitvectors (self || other)
    /// The result size is self.size() + other.size()
    /// self becomes the high bits, other becomes the low bits
    pub fn concat(&self, other: &Self) -> Self {
        let new_size = self.size() + other.size();

        match (self, other) {
            (
                Self::Concrete { value: a, .. },
                Self::Concrete {
                    value: b,
                    size: b_size,
                },
            ) => {
                // Shift a left by b_size bits and OR with b
                let shifted = a << (*b_size as usize);
                Self::from_biguint(shifted | b, new_size)
            }
            (Self::Symbolic { value: a, .. }, Self::Symbolic { value: b, .. }) => {
                Self::from_z3(a.concat(b))
            }
            (
                Self::Concrete {
                    value: a,
                    size: a_size,
                },
                Self::Symbolic { value: b, .. },
            ) => {
                // Convert concrete to symbolic for concat
                let ctx = b.get_ctx();
                let a_bv = biguint_to_bv(ctx, a, *a_size);
                Self::from_z3(a_bv.concat(b))
            }
            (
                Self::Symbolic { value: a, .. },
                Self::Concrete {
                    value: b,
                    size: b_size,
                },
            ) => {
                // Convert concrete to symbolic for concat
                let ctx = a.get_ctx();
                let b_bv = biguint_to_bv(ctx, b, *b_size);
                Self::from_z3(a.concat(&b_bv))
            }
        }
    }

    /// Bitwise AND
    pub fn and(&self, other: &Self, ctx: &'ctx Context) -> Self {
        match (self, other) {
            (Self::Concrete { value: a, size }, Self::Concrete { value: b, .. }) => {
                Self::from_biguint(a & b, *size)
            }
            _ => Self::from_z3(self.as_z3(ctx).bvand(&other.as_z3(ctx))),
        }
    }

    /// Bitwise OR
    pub fn or(&self, other: &Self, ctx: &'ctx Context) -> Self {
        match (self, other) {
            (Self::Concrete { value: a, size }, Self::Concrete { value: b, .. }) => {
                Self::from_biguint(a | b, *size)
            }
            _ => Self::from_z3(self.as_z3(ctx).bvor(&other.as_z3(ctx))),
        }
    }

    /// Bitwise XOR
    pub fn xor(&self, other: &Self, ctx: &'ctx Context) -> Self {
        match (self, other) {
            (Self::Concrete { value: a, size }, Self::Concrete { value: b, .. }) => {
                Self::from_biguint(a ^ b, *size)
            }
            _ => Self::from_z3(self.as_z3(ctx).bvxor(&other.as_z3(ctx))),
        }
    }

    /// Compare equality
    pub fn eq(&self, other: &Self, ctx: &'ctx Context) -> CbseBool<'ctx> {
        match (self, other) {
            (Self::Concrete { value: a, .. }, Self::Concrete { value: b, .. }) => {
                CbseBool::Concrete(a == b)
            }
            _ => CbseBool::from_z3(self.as_z3(ctx)._eq(&other.as_z3(ctx))),
        }
    }

    /// Unsigned less than
    pub fn ult(&self, other: &Self, ctx: &'ctx Context) -> CbseBool<'ctx> {
        match (self, other) {
            (Self::Concrete { value: a, .. }, Self::Concrete { value: b, .. }) => {
                CbseBool::Concrete(a < b)
            }
            _ => CbseBool::from_z3(self.as_z3(ctx).bvult(&other.as_z3(ctx))),
        }
    }

    /// Unsigned greater than
    pub fn ugt(&self, other: &Self, ctx: &'ctx Context) -> CbseBool<'ctx> {
        match (self, other) {
            (Self::Concrete { value: a, .. }, Self::Concrete { value: b, .. }) => {
                CbseBool::Concrete(a > b)
            }
            _ => CbseBool::from_z3(self.as_z3(ctx).bvugt(&other.as_z3(ctx))),
        }
    }

    /// Unsigned less or equal
    pub fn ule(&self, other: &Self, ctx: &'ctx Context) -> CbseBool<'ctx> {
        match (self, other) {
            (Self::Concrete { value: a, .. }, Self::Concrete { value: b, .. }) => {
                CbseBool::Concrete(a <= b)
            }
            _ => CbseBool::from_z3(self.as_z3(ctx).bvule(&other.as_z3(ctx))),
        }
    }

    /// Unsigned greater or equal
    pub fn uge(&self, other: &Self, ctx: &'ctx Context) -> CbseBool<'ctx> {
        match (self, other) {
            (Self::Concrete { value: a, .. }, Self::Concrete { value: b, .. }) => {
                CbseBool::Concrete(a >= b)
            }
            _ => CbseBool::from_z3(self.as_z3(ctx).bvuge(&other.as_z3(ctx))),
        }
    }

    /// Extract a byte by index (0 is most significant)
    pub fn byte(&self, idx: usize, ctx: &'ctx Context, output_size: u32) -> Self {
        let size = self.size();
        let byte_len = (size as usize + 7) / 8;

        if idx >= byte_len {
            return Self::from_u64(0, output_size);
        }

        match self {
            Self::Concrete { .. } => {
                let bytes = self.to_bytes();
                let byte_val = bytes[idx] as u64;
                let mut result = Self::from_u64(byte_val, 8);
                if output_size > 8 {
                    result = result.zero_extend(output_size, ctx);
                } else if output_size < 8 {
                    result = result.truncate(output_size, ctx);
                }
                result
            }
            Self::Symbolic { .. } => {
                let effective_idx = byte_len - 1 - idx;
                let lo = (effective_idx * 8) as u32;
                let hi = lo + 7;
                let mut result = Self::from_z3(self.as_z3(ctx).extract(hi, lo));
                if output_size > 8 {
                    result = result.zero_extend(output_size, ctx);
                } else if output_size < 8 {
                    result = result.truncate(output_size, ctx);
                }
                result
            }
        }
    }

    /// Extract a slice of bytes from the bitvector
    ///
    /// This extracts `size_bytes` bytes starting at byte offset `offset`.
    /// Zero-pads if the extraction goes out of bounds.
    ///
    /// # Arguments
    /// * `offset` - Byte offset to start extraction (0-indexed from left/MSB)
    /// * `size_bytes` - Number of bytes to extract
    /// * `ctx` - Z3 context for symbolic operations
    ///
    /// # Returns
    /// A new bitvector of size `size_bytes * 8` bits
    ///
    /// # Example
    /// ```ignore
    /// let bv = CbseBitVec::from_u64(0x12345678, 32);  // 4 bytes
    /// let extracted = bv.extract_bytes(1, 2, &ctx);  // Extract bytes 1-2
    /// // Result: 0x3456 (16 bits)
    /// ```
    pub fn extract_bytes(
        &self,
        offset: usize,
        size_bytes: usize,
        ctx: &'ctx Context,
    ) -> CbseResult<Self> {
        if size_bytes == 0 {
            return Ok(Self::from_u64(0, 0));
        }

        let size_bits = (size_bytes * 8) as u32;
        let data_byte_len = self.size_bytes();

        // If entirely out of bounds, return zero-padded result
        if offset >= data_byte_len {
            return Ok(Self::from_u64(0, size_bits));
        }

        let available_bytes = data_byte_len - offset;

        if available_bytes >= size_bytes {
            // Fully in bounds - extract directly
            match self {
                Self::Concrete { .. } => {
                    let bytes = self.to_bytes();
                    let extracted = &bytes[offset..offset + size_bytes];

                    // Convert bytes to BigUint (big-endian)
                    let mut value = BigUint::zero();
                    for &byte in extracted {
                        value = (value << 8) + BigUint::from(byte);
                    }

                    Ok(Self::Concrete {
                        value,
                        size: size_bits,
                    })
                }
                Self::Symbolic { .. } => {
                    // Extract from symbolic bitvector
                    // Bits are indexed from LSB (0) to MSB (size-1)
                    // But bytes are indexed from MSB
                    let bit_len = self.size() as usize;
                    let start_bit = (data_byte_len - offset - size_bytes) * 8;
                    let end_bit = start_bit + (size_bytes * 8);

                    if end_bit > bit_len {
                        // Need zero padding
                        let available_bits = bit_len - start_bit;
                        let hi = (bit_len - start_bit - 1) as u32;
                        let lo = 0u32;
                        let extracted = self.as_z3(ctx).extract(hi, lo);
                        let result = Self::from_z3(extracted);

                        // Zero-extend to desired size
                        let padding_bits = (size_bytes * 8) - available_bits;
                        Ok(result.zero_extend(size_bits, ctx))
                    } else {
                        let hi = (end_bit - 1) as u32;
                        let lo = start_bit as u32;
                        Ok(Self::from_z3(self.as_z3(ctx).extract(hi, lo)))
                    }
                }
            }
        } else {
            // Partially out of bounds - need zero padding
            let padding_bytes = size_bytes - available_bytes;

            // Extract available bytes
            let available = self.extract_bytes(offset, available_bytes, ctx)?;

            // Concatenate with zero padding
            let padding = Self::from_u64(0, (padding_bytes * 8) as u32);

            // Concat: available || padding (padding goes to LSB)
            match (&available, &padding) {
                (Self::Concrete { value: v1, .. }, Self::Concrete { .. }) => {
                    let shifted = v1 << (padding_bytes * 8);
                    Ok(Self::Concrete {
                        value: shifted,
                        size: size_bits,
                    })
                }
                _ => {
                    // Use Z3 concat for symbolic
                    let z3_available = available.as_z3(ctx);
                    let z3_padding = padding.as_z3(ctx);
                    Ok(Self::from_z3(z3_available.concat(&z3_padding)))
                }
            }
        }
    }

    /// Get the size in bytes (rounded up)
    pub fn size_bytes(&self) -> usize {
        (self.size() as usize + 7) / 8
    }

    /// Signed less than
    pub fn slt(&self, other: &Self, ctx: &'ctx Context) -> CbseBool<'ctx> {
        match (self, other) {
            (Self::Concrete { value: a, size }, Self::Concrete { value: b, .. }) => {
                let sign_bit = BigUint::one() << (size - 1);
                let max_val: BigUint = &sign_bit << 1;
                let a_signed = if a >= &sign_bit {
                    -((max_val.clone() - a).to_i64().unwrap_or(i64::MIN))
                } else {
                    a.to_i64().unwrap_or(i64::MAX)
                };
                let b_signed = if b >= &sign_bit {
                    -((max_val - b).to_i64().unwrap_or(i64::MIN))
                } else {
                    b.to_i64().unwrap_or(i64::MAX)
                };
                CbseBool::Concrete(a_signed < b_signed)
            }
            _ => CbseBool::from_z3(self.as_z3(ctx).bvslt(&other.as_z3(ctx))),
        }
    }

    /// Signed greater than
    pub fn sgt(&self, other: &Self, ctx: &'ctx Context) -> CbseBool<'ctx> {
        match (self, other) {
            (Self::Concrete { value: a, size }, Self::Concrete { value: b, .. }) => {
                let sign_bit = BigUint::one() << (size - 1);
                let max_val: BigUint = &sign_bit << 1;
                let a_signed = if a >= &sign_bit {
                    -((max_val.clone() - a).to_i64().unwrap_or(i64::MIN))
                } else {
                    a.to_i64().unwrap_or(i64::MAX)
                };
                let b_signed = if b >= &sign_bit {
                    -((max_val - b).to_i64().unwrap_or(i64::MIN))
                } else {
                    b.to_i64().unwrap_or(i64::MAX)
                };
                CbseBool::Concrete(a_signed > b_signed)
            }
            _ => CbseBool::from_z3(self.as_z3(ctx).bvsgt(&other.as_z3(ctx))),
        }
    }

    /// Bitwise NOT
    pub fn not(&self, ctx: &'ctx Context) -> Self {
        match self {
            Self::Concrete { value, size } => {
                let mask = mask(*size);
                Self::from_biguint((&mask) ^ value, *size)
            }
            _ => Self::from_z3(self.as_z3(ctx).bvnot()),
        }
    }

    /// Shift left
    pub fn shl(&self, shift: &Self, ctx: &'ctx Context) -> Self {
        match (self, shift) {
            (
                Self::Concrete { value, size },
                Self::Concrete {
                    value: shift_amt, ..
                },
            ) => {
                if let Some(shift_u32) = shift_amt.to_u32() {
                    if shift_u32 >= *size {
                        Self::from_u64(0, *size)
                    } else {
                        let mask = (BigUint::one() << size) - BigUint::one();
                        Self::from_biguint((value << shift_u32) & mask, *size)
                    }
                } else {
                    Self::from_u64(0, *size)
                }
            }
            _ => Self::from_z3(self.as_z3(ctx).bvshl(&shift.as_z3(ctx))),
        }
    }

    /// Logical shift right
    pub fn shr(&self, shift: &Self, ctx: &'ctx Context) -> Self {
        self.lshr(shift, ctx)
    }

    /// Logical shift right (alias)
    pub fn lshr(&self, shift: &Self, ctx: &'ctx Context) -> Self {
        match (self, shift) {
            (
                Self::Concrete { value, size },
                Self::Concrete {
                    value: shift_amt, ..
                },
            ) => {
                if let Some(shift_u32) = shift_amt.to_u32() {
                    if shift_u32 >= *size {
                        Self::from_u64(0, *size)
                    } else {
                        Self::from_biguint(value >> shift_u32, *size)
                    }
                } else {
                    Self::from_u64(0, *size)
                }
            }
            _ => Self::from_z3(self.as_z3(ctx).bvlshr(&shift.as_z3(ctx))),
        }
    }

    /// Arithmetic shift right (preserves sign)
    pub fn sar(&self, shift: &Self, ctx: &'ctx Context) -> Self {
        self.ashr(shift, ctx)
    }

    /// Arithmetic shift right (alias)
    pub fn ashr(&self, shift: &Self, ctx: &'ctx Context) -> Self {
        match (self, shift) {
            (
                Self::Concrete { value, size },
                Self::Concrete {
                    value: shift_amt, ..
                },
            ) => {
                let sign_bit = BigUint::one() << (size - 1);
                let is_negative = value >= &sign_bit;

                if let Some(shift_u32) = shift_amt.to_u32() {
                    if shift_u32 >= *size {
                        if is_negative {
                            // All 1s
                            let mask = (BigUint::one() << size) - BigUint::one();
                            Self::from_biguint(mask, *size)
                        } else {
                            Self::from_u64(0, *size)
                        }
                    } else {
                        let shifted = value >> shift_u32;
                        if is_negative {
                            // Fill with 1s from the left
                            let fill_mask = ((BigUint::one() << shift_u32) - BigUint::one())
                                << (size - shift_u32);
                            Self::from_biguint(shifted | fill_mask, *size)
                        } else {
                            Self::from_biguint(shifted, *size)
                        }
                    }
                } else {
                    if is_negative {
                        let mask = (BigUint::one() << size) - BigUint::one();
                        Self::from_biguint(mask, *size)
                    } else {
                        Self::from_u64(0, *size)
                    }
                }
            }
            _ => Self::from_z3(self.as_z3(ctx).bvashr(&shift.as_z3(ctx))),
        }
    }

    /// Create a bit vector from 32-byte slice (U256 format)
    pub fn from_u256(_ctx: &'ctx Context, bytes: &[u8]) -> Self {
        if bytes.len() < 32 {
            return Self::from_u64(0, 256);
        }
        let value = BigUint::from_bytes_be(&bytes[0..32]);
        Self::Concrete { value, size: 256 }
    }

    /// Convert to bytes (big-endian). Symbolic values are zero-filled to match the bit-width.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Concrete { value, size } => {
                let bytes = value.to_bytes_be();
                let target_len = (*size as usize + 7) / 8;
                if target_len == 0 {
                    return Vec::new();
                }

                if bytes.len() < target_len {
                    let mut result = vec![0u8; target_len];
                    result[target_len - bytes.len()..].copy_from_slice(&bytes);
                    result
                } else if bytes.len() > target_len {
                    bytes[bytes.len() - target_len..].to_vec()
                } else {
                    bytes
                }
            }
            Self::Symbolic { size, .. } => vec![0u8; (*size as usize + 7) / 8],
        }
    }

    /// Attempt to convert to bytes, returning an error if value is symbolic.
    pub fn to_concrete_bytes(&self) -> CbseResult<Vec<u8>> {
        match self {
            Self::Concrete { .. } => Ok(self.to_bytes()),
            Self::Symbolic { .. } => Err(CbseException::NotConcrete(
                "Cannot convert symbolic bitvector to bytes".to_string(),
            )),
        }
    }
}

impl<'ctx> fmt::Debug for CbseBitVec<'ctx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Concrete { value, size } => write!(f, "BV({}, {})", value, size),
            Self::Symbolic { value, size } => write!(f, "BV({}, {})", value, size),
        }
    }
}

/// Common constants
pub const ZERO: u64 = 0;
pub const ONE: u64 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_power_of_two() {
        assert!(is_power_of_two(1));
        assert!(is_power_of_two(2));
        assert!(is_power_of_two(256));
        assert!(!is_power_of_two(0));
        assert!(!is_power_of_two(3));
    }

    #[test]
    fn test_concrete_operations() {
        let a = CbseBitVec::from_u64(10, 256);
        let b = CbseBitVec::from_u64(5, 256);

        let cfg = z3::Config::new();
        let ctx = Context::new(&cfg);

        let sum = a.add(&b, &ctx);
        assert_eq!(sum.as_u64().unwrap(), 15);
    }
}
