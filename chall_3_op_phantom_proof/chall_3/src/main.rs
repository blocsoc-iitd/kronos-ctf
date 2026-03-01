//! Phantom STARK CLI: prove, verify, and check Rescue-Prime hash proofs.

use clap::{Parser, Subcommand};
use phantom_stark::field::BabyBear;
use phantom_stark::hash::rescue::rescue_hash;
use phantom_stark::stark::{prove, verify};
use phantom_stark::stark::types::StarkProof;
#[derive(Parser)]
#[command(name = "phantom-stark")]
#[command(about = "STARK prover/verifier for Rescue-Prime hash")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compute the Rescue-Prime hash of an input
    Hash {
        /// Input as two comma-separated integers (e.g., "0,0")
        #[arg(long)]
        input: String,
    },
    /// Generate a STARK proof
    Prove {
        /// Input as two comma-separated integers
        #[arg(long)]
        input: String,
        /// Output as two comma-separated integers
        #[arg(long)]
        output: String,
        /// Output file for the proof
        #[arg(short, long, default_value = "proof.bin")]
        out: String,
    },
    /// Verify a STARK proof
    Verify {
        /// Input as two comma-separated integers
        #[arg(long)]
        input: String,
        /// Output as two comma-separated integers
        #[arg(long)]
        output: String,
        /// Proof file
        #[arg(long)]
        proof: String,
    },
}

fn parse_pair(s: &str) -> Result<[BabyBear; 2], String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err("Expected two comma-separated integers".to_string());
    }
    let a = parts[0]
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("Invalid first value: {}", e))?;
    let b = parts[1]
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("Invalid second value: {}", e))?;
    Ok([BabyBear::new(a % phantom_stark::field::MODULUS), BabyBear::new(b % phantom_stark::field::MODULUS)])
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Hash { input } => {
            let inp = parse_pair(&input).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let out = rescue_hash(inp);
            println!(
                "Rescue-Prime({}, {}) = ({}, {})",
                inp[0].to_canonical(),
                inp[1].to_canonical(),
                out[0].to_canonical(),
                out[1].to_canonical()
            );
        }
        Commands::Prove { input, output, out } => {
            let inp = parse_pair(&input).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let outp = parse_pair(&output).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            println!("Generating proof...");
            let proof = prove(inp, outp);
            let bytes = proof.to_bytes();
            std::fs::write(&out, &bytes).unwrap_or_else(|e| {
                eprintln!("Error writing proof: {}", e);
                std::process::exit(1);
            });
            println!("Proof written to {} ({} bytes)", out, bytes.len());
        }
        Commands::Verify { input, output, proof } => {
            let inp = parse_pair(&input).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let outp = parse_pair(&output).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let bytes = std::fs::read(&proof).unwrap_or_else(|e| {
                eprintln!("Error reading proof: {}", e);
                std::process::exit(1);
            });
            let stark_proof = StarkProof::from_bytes(&bytes).unwrap_or_else(|| {
                eprintln!("Error: invalid proof format");
                std::process::exit(1);
            });
            match verify(&stark_proof, inp, outp) {
                Ok(()) => println!("Proof is VALID."),
                Err(e) => {
                    println!("Proof is INVALID: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
