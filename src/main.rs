mod air;
mod columns;
mod config;
mod generation;

use p3_uni_stark::{prove, verify};
use tracing_forest::ForestLayer;
use tracing_forest::util::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

use crate::air::AgeProofAir;
use crate::config::{MyStarkConfig, Val, stark_config};
use crate::generation::generate_age_proof_trace;

fn main() {
    // Setup tracing for proof timing info
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    Registry::default()
        .with(env_filter)
        .with(ForestLayer::default())
        .init();

    println!("=== ZKP Age Proof (Plonky3 STARK) ===\n");

    // --- Example 1: Prouver qu'un adulte a >= 18 ans ---
    println!("--- Test 1: Adulte (ne en 1995, annee 2026) ---");
    run_age_proof(1995, 2026);

    // --- Example 2: Prouver qu'un mineur a < 18 ans ---
    println!("\n--- Test 2: Mineur (ne en 2015, annee 2026) ---");
    run_age_proof(2015, 2026);

    // --- Example 3: Cas limite - exactement 18 ans ---
    println!("\n--- Test 3: Exactement 18 ans (ne en 2008, annee 2026) ---");
    run_age_proof(2008, 2026);

    // --- Example 4: Cas limite - 17 ans ---
    println!("\n--- Test 4: 17 ans (ne en 2009, annee 2026) ---");
    run_age_proof(2009, 2026);

    println!("\n=== Tous les tests passes! ===");
}

fn run_age_proof(year_of_birth: u32, current_year: u32) {
    let age = current_year as i32 - year_of_birth as i32;
    let is_adult = age >= 18;
    let trace_height = 1 << 3; // 8 rows

    println!("  Year of birth: {year_of_birth} (SECRET - non revele)");
    println!("  Current year:  {current_year} (PUBLIC)");

    // Generate trace and public values
    let (trace, pis) = generate_age_proof_trace::<Val>(year_of_birth, current_year, trace_height);

    println!(
        "  Public values:  age_ok={}, current_year={}",
        if is_adult { 1 } else { 0 },
        current_year
    );

    // Build STARK config
    let config = stark_config();

    // Generate proof
    println!("  Generating STARK proof...");
    let proof = prove(&config, &AgeProofAir, trace, &pis);

    // Serialize the proof (this is what gets sent to the blockchain)
    let proof_bytes = postcard::to_allocvec(&proof).unwrap();
    println!("  Proof size: {} bytes", proof_bytes.len());

    // === VERIFIER SIDE ===
    // Deserialize and verify (simulates what the verifier does with on-chain bytes)
    let deserialized_proof: p3_uni_stark::Proof<MyStarkConfig> =
        postcard::from_bytes(&proof_bytes).expect("Deserialization failed!");

    println!("  Verifying deserialized proof...");
    let verifier_config = stark_config();
    verify(&verifier_config, &AgeProofAir, &deserialized_proof, &pis)
        .expect("Verification failed!");

    println!("  VERIFIED: age >= 18 = {is_adult} (without revealing year of birth!)");
}
