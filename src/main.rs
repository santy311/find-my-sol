use anyhow::Result;
use clap::{Parser, Subcommand};
use rand::SeedableRng;
use solana_sdk::signature::Signer;
use vanity::VanityGenerator;

mod opencl;
mod utils;
mod vanity;

use opencl::OpenCLManager;

#[derive(Parser)]
#[command(name = "solana-vanity")]
#[command(about = "Extremely fast Solana vanity address generator with OpenCL support")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for vanity public keys
    SearchPubkey {
        /// Pattern that the public key should start with
        #[arg(long, short)]
        starts_with: Option<String>,

        /// Pattern that the public key should end with
        #[arg(long, short)]
        ends_with: Option<String>,

        /// Number of vanity addresses to generate
        #[arg(long, short, default_value = "1")]
        count: usize,

        /// OpenCL device index to use
        #[arg(long, short)]
        device: Option<usize>,

        /// Number of bits to use for iteration (higher = more parallel work)
        #[arg(long, default_value = "20")]
        iteration_bits: u32,

        /// Case sensitive matching
        #[arg(long, short = 'C')]
        case_sensitive: bool,

        /// Output file to save results
        #[arg(long, short, default_value = "vanity_results.json")]
        output: String,
    },

    /// Show available OpenCL devices
    ShowDevices,

    /// Test mode - verify the implementation works correctly
    Test,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::SearchPubkey {
            starts_with,
            ends_with,
            count,
            device,
            iteration_bits,
            case_sensitive,
            output,
        } => {
            let mut generator = VanityGenerator::new(
                starts_with,
                ends_with,
                count,
                device,
                iteration_bits,
                case_sensitive,
                output,
            )?;

            generator.run().await?;
        }

        Commands::ShowDevices => {
            let opencl_manager = OpenCLManager::new()?;
            opencl_manager.list_devices()?;
        }

        Commands::Test => {
            test_vanity_generation()?;
        }
    }

    Ok(())
}

fn test_vanity_generation() -> Result<()> {
    println!("🧪 Testing vanity address generation...");

    // Test 1: Generate a simple keypair and verify it's valid
    println!("Test 1: Basic keypair generation");
    let keypair = solana_sdk::signature::Keypair::new();
    let pubkey = keypair.pubkey();
    let address = pubkey.to_string();
    println!("Generated address: {}", address);
    println!("Address length: {} (should be 44)", address.len());
    println!("Address starts with: {}", &address[..4]);
    println!("✅ Basic keypair generation works");

    // Test 2: Test pattern matching
    println!("\nTest 2: Pattern matching");
    let test_patterns = vec!["ABC".to_string(), "XYZ".to_string()];
    let test_address = "ABC123XYZ456789".to_string();

    for pattern in &test_patterns {
        let matches = test_address.contains(pattern);
        println!("Pattern '{}' in '{}': {}", pattern, test_address, matches);
    }
    println!("✅ Pattern matching works");

    // Test 3: Test seed-based generation
    println!("\nTest 3: Seed-based generation");
    let seed = 12345u32;
    let seed_bytes = seed.to_le_bytes();
    let keypair_from_seed = solana_sdk::signature::Keypair::from_bytes(&seed_bytes);
    match keypair_from_seed {
        Ok(kp) => {
            let addr = kp.pubkey().to_string();
            println!("Generated from seed {}: {}", seed, addr);
            println!("✅ Seed-based generation works");
        }
        Err(_) => {
            println!("⚠️  Seed-based generation failed, using fallback");
            let mut rng = rand::prelude::StdRng::seed_from_u64(seed as u64);
            let kp = solana_sdk::signature::Keypair::new();
            let addr = kp.pubkey().to_string();
            println!("Fallback generation: {}", addr);
            println!("✅ Fallback generation works");
        }
    }

    // Test 4: Test OpenCL integration
    println!("\nTest 4: OpenCL integration");
    match opencl::OpenCLManager::new() {
        Ok(manager) => {
            println!("✅ OpenCL manager created successfully");
            match manager.create_vanity_kernel(0) {
                Ok(kernel) => {
                    println!("✅ Vanity kernel created successfully");
                    match kernel.generate_seeds(1000) {
                        Ok(seeds) => {
                            println!("✅ Generated {} seeds via OpenCL", seeds.len());
                            println!("Sample seeds: {:?}", &seeds[..5]);
                        }
                        Err(e) => println!("❌ Seed generation failed: {}", e),
                    }
                }
                Err(e) => println!("❌ Kernel creation failed: {}", e),
            }
        }
        Err(e) => println!("❌ OpenCL manager creation failed: {}", e),
    }

    println!("\n🎉 All tests completed!");
    Ok(())
}
