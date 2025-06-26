use anyhow::Result;
use bs58;
use rand::Rng;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
pub struct VanityResult {
    pub public_key: String,
    pub private_key: String,
    pub pattern_matched: String,
    pub attempts: u64,
    pub found_at: chrono::DateTime<chrono::Utc>,
}

pub fn generate_keypair_from_seed(seed: u32) -> Keypair {
    let mut rng = rand::thread_rng();
    let mut seed_bytes = [0u8; 32];
    rng.fill(&mut seed_bytes);

    // Use the provided seed to influence the generation
    seed_bytes[0] = (seed & 0xFF) as u8;
    seed_bytes[1] = ((seed >> 8) & 0xFF) as u8;
    seed_bytes[2] = ((seed >> 16) & 0xFF) as u8;
    seed_bytes[3] = ((seed >> 24) & 0xFF) as u8;

    Keypair::from_bytes(&seed_bytes).unwrap_or_else(|_| {
        // Fallback to random generation if seed-based fails
        Keypair::new()
    })
}

pub fn check_pattern_match(
    pubkey: &Pubkey,
    starts_with: &Option<String>,
    ends_with: &Option<String>,
    case_sensitive: bool,
) -> bool {
    let pubkey_str = pubkey.to_string();

    let check_starts = if let Some(pattern) = starts_with {
        if case_sensitive {
            pubkey_str.starts_with(pattern)
        } else {
            let pubkey_lower = pubkey_str.to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            pubkey_lower.starts_with(&pattern_lower)
        }
    } else {
        true
    };

    let check_ends = if let Some(pattern) = ends_with {
        if case_sensitive {
            pubkey_str.ends_with(pattern)
        } else {
            let pubkey_lower = pubkey_str.to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            pubkey_lower.ends_with(&pattern_lower)
        }
    } else {
        true
    };

    check_starts && check_ends
}

pub fn save_results(results: &[VanityResult], output_path: &str) -> Result<()> {
    let output = serde_json::to_string_pretty(results)?;
    fs::write(output_path, output)?;
    println!("Saved {} results to {}", results.len(), output_path);
    Ok(())
}

pub fn load_existing_results(output_path: &str) -> Result<Vec<VanityResult>> {
    if Path::new(output_path).exists() {
        let content = fs::read_to_string(output_path)?;
        let results: Vec<VanityResult> = serde_json::from_str(&content)?;
        Ok(results)
    } else {
        Ok(Vec::new())
    }
}

pub fn generate_random_seeds(count: usize) -> Vec<u32> {
    let mut rng = rand::thread_rng();
    (0..count).map(|_| rng.gen()).collect()
}

pub fn format_attempts(attempts: u64) -> String {
    if attempts >= 1_000_000_000 {
        format!("{:.2}B", attempts as f64 / 1_000_000_000.0)
    } else if attempts >= 1_000_000 {
        format!("{:.2}M", attempts as f64 / 1_000_000.0)
    } else if attempts >= 1_000 {
        format!("{:.2}K", attempts as f64 / 1_000.0)
    } else {
        attempts.to_string()
    }
}

pub fn calculate_probability(starts_with: &Option<String>, ends_with: &Option<String>) -> f64 {
    let mut total_length = 0;

    if let Some(pattern) = starts_with {
        total_length += pattern.len();
    }

    if let Some(pattern) = ends_with {
        total_length += pattern.len();
    }

    if total_length == 0 {
        return 1.0;
    }

    // Base58 alphabet has 58 characters
    let base58_chars: f64 = 58.0;

    // Probability of matching a specific pattern
    let probability = 1.0 / base58_chars.powi(total_length as i32);

    probability
}

pub fn estimate_attempts_needed(starts_with: &Option<String>, ends_with: &Option<String>) -> u64 {
    let probability = calculate_probability(starts_with, ends_with);

    // For 50% chance of finding a match
    let attempts = (0.693 / probability) as u64;

    attempts
}
