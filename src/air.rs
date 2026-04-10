use core::borrow::Borrow;

use p3_air::{Air, AirBuilder, BaseAir, WindowAccess};
use p3_field::PrimeCharacteristicRing;

use crate::columns::{AgeProofCols, NUM_AGE_PROOF_COLS};

/// AIR (Arithmetized Intermediate Representation) for proving age >= 18.
///
/// This circuit takes:
///   - Private input: year_of_birth (kept secret in the trace)
///   - Public inputs: [age_ok, current_year]
///
/// And proves that `age_ok` is consistent with the actual age
/// computed from `current_year - year_of_birth`, without revealing
/// the year of birth.
pub struct AgeProofAir;

impl<F> BaseAir<F> for AgeProofAir {
    fn width(&self) -> usize {
        NUM_AGE_PROOF_COLS
    }

    fn num_public_values(&self) -> usize {
        // pis[0] = age_ok (0 or 1)
        // pis[1] = current_year
        2
    }

    fn max_constraint_degree(&self) -> Option<usize> {
        // Highest degree constraint:
        //   when_first_row (degree 1) * when(age_ok) (degree 1) * assert_eq (degree 1) = 3
        // Boolean checks:
        //   when_first_row (degree 1) * bool_check (degree 2) = 3
        Some(3)
    }

    fn main_next_row_columns(&self) -> Vec<usize> {
        // We never access the next row, so return empty.
        vec![]
    }
}

impl<AB: AirBuilder> Air<AB> for AgeProofAir {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();

        let local: &AgeProofCols<AB::Var> = main.current_slice().borrow();

        // Copy public values into locals to release the immutable borrow on builder.
        let pis = builder.public_values();
        let pi_age_ok = pis[0];
        let pi_current_year = pis[1];

        // All constraints are gated to the first row only.
        // Other rows are zero-padded and unconstrained.
        let mut first = builder.when_first_row();

        // --- Constraint 1: public values must match trace values ---
        first.assert_eq(local.age_ok, pi_age_ok);
        first.assert_eq(local.current_year, pi_current_year);

        // --- Constraint 2: age_ok must be boolean (0 or 1) ---
        first.assert_bool(local.age_ok);

        // --- Constraint 3: each diff bit must be boolean ---
        for &bit in &local.diff_bits {
            first.assert_bool(bit);
        }

        // --- Constraint 4: reconstruct diff from its binary decomposition ---
        // diff = sum_{i=0}^{6} diff_bits[i] * 2^i
        let diff = local
            .diff_bits
            .iter()
            .enumerate()
            .fold(AB::Expr::ZERO, |acc, (i, &bit)| {
                acc + bit.into() * AB::Expr::from_u64(1u64 << i)
            });

        // --- Constraint 5: age relationship ---
        //
        // If age_ok = 1 (adult):
        //   year_of_birth + diff + 18 == current_year
        //   i.e. diff = (current_year - year_of_birth) - 18 = age - 18 >= 0
        //
        // If age_ok = 0 (minor):
        //   current_year + diff + 1 == year_of_birth + 18
        //   i.e. diff = (year_of_birth + 18) - current_year - 1 = 17 - age >= 0
        //
        // The range check on diff (via bit decomposition) guarantees diff >= 0.

        let eighteen = AB::Expr::from_u32(18);

        // Case age_ok = 1: year_of_birth + diff + 18 = current_year
        first.when(local.age_ok).assert_eq(
            local.year_of_birth.into() + diff.clone() + eighteen.clone(),
            local.current_year,
        );

        // Case age_ok = 0: current_year + diff + 1 = year_of_birth + 18
        first
            .when_ne(local.age_ok, AB::Expr::ONE)
            .assert_eq(
                local.current_year.into() + diff + AB::Expr::ONE,
                local.year_of_birth.into() + eighteen,
            );
    }
}
