use anyhow::Result;
use clap::{Parser, Subcommand};

mod opencl;
mod utils;
mod vanity;

use opencl::OpenCLManager;
use vanity::VanityGenerator;

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
    }

    Ok(())
}
