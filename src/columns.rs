use core::borrow::{Borrow, BorrowMut};
use core::mem::size_of;

/// Number of bits used to decompose the age difference for range checking.
/// 7 bits covers [0, 127], more than enough for age differences.
pub const DIFF_BITS: usize = 7;

/// Trace columns for proving age >= 18.
///
/// The prover fills one meaningful row; the remaining rows are zero-padded.
/// Only the first row is constrained.
#[repr(C)]
pub struct AgeProofCols<T> {
    /// Year of birth (SECRET - stays in the trace, never revealed)
    pub year_of_birth: T,

    /// Current year (PUBLIC - also exposed as a public value)
    pub current_year: T,

    /// 1 if age >= 18, 0 otherwise (PUBLIC - also exposed as a public value)
    pub age_ok: T,

    /// Binary decomposition of the age difference for range checking.
    ///
    /// If age_ok = 1: diff = age - 18            (proves age >= 18)
    /// If age_ok = 0: diff = 17 - age = 18 - 1 - age  (proves age < 18)
    pub diff_bits: [T; DIFF_BITS],
}

pub const NUM_AGE_PROOF_COLS: usize = size_of::<AgeProofCols<u8>>();

impl<T> Borrow<AgeProofCols<T>> for [T] {
    fn borrow(&self) -> &AgeProofCols<T> {
        debug_assert_eq!(self.len(), NUM_AGE_PROOF_COLS);
        let (prefix, shorts, suffix) = unsafe { self.align_to::<AgeProofCols<T>>() };
        debug_assert!(prefix.is_empty(), "Alignment should match");
        debug_assert!(suffix.is_empty(), "Alignment should match");
        debug_assert_eq!(shorts.len(), 1);
        &shorts[0]
    }
}

impl<T> BorrowMut<AgeProofCols<T>> for [T] {
    fn borrow_mut(&mut self) -> &mut AgeProofCols<T> {
        debug_assert_eq!(self.len(), NUM_AGE_PROOF_COLS);
        let (prefix, shorts, suffix) = unsafe { self.align_to_mut::<AgeProofCols<T>>() };
        debug_assert!(prefix.is_empty(), "Alignment should match");
        debug_assert!(suffix.is_empty(), "Alignment should match");
        debug_assert_eq!(shorts.len(), 1);
        &mut shorts[0]
    }
}
