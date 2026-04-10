use p3_field::PrimeCharacteristicRing;
use p3_matrix::dense::RowMajorMatrix;

use crate::columns::{DIFF_BITS, NUM_AGE_PROOF_COLS};

/// Generate the execution trace and public values for an age proof.
///
/// # Arguments
/// - `year_of_birth`: The secret year of birth (e.g. 1995)
/// - `current_year`: The current year (e.g. 2026)
/// - `trace_height`: Number of rows in the trace (must be a power of 2, >= 8)
///
/// # Returns
/// `(trace, public_values)` where:
/// - `trace`: a `RowMajorMatrix` with `trace_height` rows
/// - `public_values`: `[age_ok, current_year]`
pub fn generate_age_proof_trace<F: PrimeCharacteristicRing + Send + Sync>(
    year_of_birth: u32,
    current_year: u32,
    trace_height: usize,
) -> (RowMajorMatrix<F>, Vec<F>) {
    assert!(trace_height.is_power_of_two(), "trace height must be a power of 2");
    assert!(trace_height >= 2, "trace height must be at least 2");

    // Compute the age and result
    let age = current_year as i32 - year_of_birth as i32;
    let age_ok: u32 = if age >= 18 { 1 } else { 0 };

    // Compute diff for the range check
    let diff: u32 = if age >= 18 {
        (age - 18) as u32
    } else {
        (17 - age) as u32
    };
    assert!(diff < (1 << DIFF_BITS), "age difference too large for {DIFF_BITS} bits");

    // Build the trace: trace_height rows, NUM_AGE_PROOF_COLS columns, all zeros initially
    let mut values = vec![F::ZERO; trace_height * NUM_AGE_PROOF_COLS];

    // Fill only the first row with actual data
    let row = &mut values[..NUM_AGE_PROOF_COLS];
    row[0] = F::from_u32(year_of_birth); // year_of_birth
    row[1] = F::from_u32(current_year);  // current_year
    row[2] = F::from_u32(age_ok);        // age_ok

    // Fill diff_bits (little-endian binary decomposition)
    for i in 0..DIFF_BITS {
        row[3 + i] = F::from_u32((diff >> i) & 1);
    }

    let trace = RowMajorMatrix::new(values, NUM_AGE_PROOF_COLS);

    // Public values: what the verifier knows
    let public_values = vec![F::from_u32(age_ok), F::from_u32(current_year)];

    (trace, public_values)
}
