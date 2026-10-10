//! Fused multiply-add where clang's arm64 build of HiGHS has one.
//!
//! The port mirrors clang: on arm64 clang contracts `a ± b*c` within an
//! expression into an FMA instruction, and the Rust writes those as
//! `x.mul_add_c(a, b)`. That is `mul_add` (one rounding) wherever the CPU
//! has FMA instructions: aarch64, and x86_64 built with FMA enabled (e.g.
//! `-C target-cpu=x86-64-v3`, as for AWS Lambda), so those builds take the
//! same paths as each other. Elsewhere, or with the `no_fma` feature, it is
//! `x * a + b` (two roundings), as in a C++ build without contraction
//! (gcc with `-ffp-contract=off`, the CI oracle, or the default x86_64
//! build, which has no FMA instructions). `f64::mul_add` without hardware
//! FMA would be a slow software `fma()`, hence the target condition.

/// Whether `mul_add_c` fuses
pub const FUSED: bool = cfg!(all(
    any(target_arch = "aarch64", all(target_arch = "x86_64", target_feature = "fma")),
    not(feature = "no_fma")
));

pub trait ClangFma {
    fn mul_add_c(self, a: Self, b: Self) -> Self;
}

impl ClangFma for f64 {
    #[inline(always)]
    fn mul_add_c(self, a: f64, b: f64) -> f64 {
        if FUSED {
            self.mul_add(a, b)
        } else {
            self * a + b
        }
    }
}
