use anyhow::{anyhow, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use indicatif::{ProgressBar, ProgressStyle};
use rand::Rng;
use rayon::prelude::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::opencl::OpenCLManager;
use crate::utils::{
    check_pattern_match, estimate_attempts_needed, format_attempts, generate_keypair_from_seed,
    generate_random_seeds, load_existing_results, save_results, VanityResult,
};

pub struct VanityGenerator {
    starts_with: Option<String>,
    ends_with: Option<String>,
    count: usize,
    device: Option<usize>,
    iteration_bits: u32,
    case_sensitive: bool,
    output_path: String,
    opencl_manager: Option<OpenCLManager>,
    results: Arc<Mutex<Vec<VanityResult>>>,
    total_attempts: Arc<Mutex<u64>>,
}

impl VanityGenerator {
    pub fn new(
        starts_with: Option<String>,
        ends_with: Option<String>,
        count: usize,
        device: Option<usize>,
        iteration_bits: u32,
        case_sensitive: bool,
        output_path: String,
    ) -> Result<Self> {
        let opencl_manager = OpenCLManager::new().ok();

        // Load existing results
        let existing_results = load_existing_results(&output_path).unwrap_or_default();
        let results = Arc::new(Mutex::new(existing_results));
        let total_attempts = Arc::new(Mutex::new(0u64));

        Ok(VanityGenerator {
            starts_with,
            ends_with,
            count,
            device,
            iteration_bits,
            case_sensitive,
            output_path,
            opencl_manager,
            results,
            total_attempts,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        println!("🚀 Starting Solana vanity address generator");
        println!(
            "Pattern: starts_with={:?}, ends_with={:?}",
            self.starts_with, self.ends_with
        );
        println!("Target count: {}", self.count);
        println!("Case sensitive: {}", self.case_sensitive);
        println!("Iteration bits: {}", self.iteration_bits);

        if let Some(device) = self.device {
            println!("Using OpenCL device: {}", device);
        } else {
            println!("Using CPU-only mode");
        }

        let estimated_attempts = estimate_attempts_needed(&self.starts_with, &self.ends_with);
        println!(
            "Estimated attempts needed: {}",
            format_attempts(estimated_attempts)
        );

        let start_time = Instant::now();

        // Check if we already have enough results
        // let current_count = {
        //     let results = self.results.lock().unwrap();
        //     results.len()
        // };

        // if current_count >= self.count {
        //     println!(
        //         "✅ Already have {} results, no need to search",
        //         current_count
        //     );
        //     self.display_results();
        //     return Ok(());
        // }

        let remaining_count = self.count; // Always search for the requested count
        println!("Need to find {} more vanity addresses", remaining_count);

        // Create progress bar
        let progress_bar = ProgressBar::new(remaining_count as u64);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

        // Start the search
        if let Some(device) = self.device {
            if let Some(ref opencl_manager) = self.opencl_manager {
                self.run_gpu_search(opencl_manager, device, remaining_count, &progress_bar)
                    .await?;
            } else {
                self.run_cpu_search(remaining_count, &progress_bar).await?;
            }
        } else {
            self.run_cpu_search(remaining_count, &progress_bar).await?;
        }

        progress_bar.finish_with_message("Search completed!");

        let elapsed = start_time.elapsed();
        let total_attempts = *self.total_attempts.lock().unwrap();
        let rate = total_attempts as f64 / elapsed.as_secs_f64();

        println!("\n🎉 Search completed!");
        println!("Total time: {:.2}s", elapsed.as_secs_f64());
        println!("Total attempts: {}", format_attempts(total_attempts));
        println!("Rate: {:.2} attempts/sec", rate);

        self.display_results();
        self.save_results()?;

        Ok(())
    }

    async fn run_cpu_search(&self, target_count: usize, progress_bar: &ProgressBar) -> Result<()> {
        let batch_size = 1_000_000; // 1M keypairs per batch
        let num_threads = num_cpus::get();

        println!("Using {} CPU threads", num_threads);

        let (tx, rx) = bounded::<VanityResult>(1000);

        // Spawn worker threads
        let mut handles = Vec::new();

        for _ in 0..num_threads {
            let tx = tx.clone();
            let starts_with = self.starts_with.clone();
            let ends_with = self.ends_with.clone();
            let case_sensitive = self.case_sensitive;
            let total_attempts = Arc::clone(&self.total_attempts);

            let handle = thread::spawn(move || {
                let mut local_attempts = 0u64;
                let mut rng = rand::thread_rng();

                loop {
                    // Generate batch of seeds
                    let seeds: Vec<u32> = (0..batch_size).map(|_| rng.gen()).collect();

                    // Process seeds in parallel
                    let found_results: Vec<VanityResult> = seeds
                        .par_iter()
                        .filter_map(|&seed| {
                            let keypair = generate_keypair_from_seed(seed);
                            let pubkey = keypair.pubkey();
                            if check_pattern_match(
                                &pubkey,
                                &starts_with,
                                &ends_with,
                                case_sensitive,
                            ) {
                                let pattern_matched = if starts_with.is_some() {
                                    starts_with.as_ref().unwrap().clone()
                                } else if ends_with.is_some() {
                                    ends_with.as_ref().unwrap().clone()
                                } else {
                                    "random".to_string()
                                };
                                Some(VanityResult {
                                    public_key: pubkey.to_string(),
                                    private_key: bs58::encode(keypair.to_bytes()).into_string(),
                                    pattern_matched,
                                    attempts: 0, // We'll update this below
                                    found_at: chrono::Utc::now(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                    // Send found results
                    for mut result in found_results {
                        // Update attempts for each result
                        local_attempts += 1;
                        result.attempts = local_attempts;
                        if tx.send(result).is_err() {
                            return; // Channel closed, exit thread
                        }
                    }

                    // Update global attempt counter
                    {
                        let mut global_attempts = total_attempts.lock().unwrap();
                        *global_attempts += batch_size as u64;
                    }
                }
            });

            handles.push(handle);
        }

        // Collect results
        let mut found_count = 0;
        while found_count < target_count {
            if let Ok(result) = rx.recv() {
                {
                    let mut results = self.results.lock().unwrap();
                    results.push(result);
                }
                found_count += 1;
                progress_bar.inc(1);

                // Save immediately
                self.save_results()?;
            }
        }

        // Clean up threads
        drop(tx);
        for handle in handles {
            let _ = handle.join();
        }

        Ok(())
    }

    async fn run_gpu_search(
        &self,
        opencl_manager: &OpenCLManager,
        device_idx: usize,
        target_count: usize,
        progress_bar: &ProgressBar,
    ) -> Result<()> {
        let kernel = opencl_manager.create_vanity_kernel(device_idx)?;
        let batch_size = 1_000_000; // 1M keypairs per batch

        println!("Using OpenCL device {} for GPU acceleration", device_idx);

        let (tx, rx) = bounded::<VanityResult>(1000);

        // Spawn GPU worker thread
        let tx_clone = tx.clone();
        let starts_with = self.starts_with.clone();
        let ends_with = self.ends_with.clone();
        let case_sensitive = self.case_sensitive;
        let total_attempts = Arc::clone(&self.total_attempts);

        let gpu_handle = thread::spawn(move || {
            let mut local_attempts = 0u64;

            loop {
                // Generate batch of seeds
                let seeds = generate_random_seeds(batch_size);

                // Use GPU to generate keypairs
                if let Ok(gpu_results) = kernel.generate_keys(
                    &seeds,
                    starts_with.as_deref().unwrap_or(""),
                    ends_with.as_deref().unwrap_or(""),
                    case_sensitive,
                ) {
                    // Process GPU results
                    for i in 0..batch_size {
                        local_attempts += 1;

                        let result_offset = i * 64;
                        let pubkey_bytes = &gpu_results[result_offset..result_offset + 32];
                        let private_key_bytes =
                            &gpu_results[result_offset + 32..result_offset + 64];

                        // Check if this is a valid result (non-zero bytes indicate match)
                        if pubkey_bytes.iter().any(|&b| b != 0) {
                            if let Ok(pubkey) = Pubkey::try_from(pubkey_bytes) {
                                let pattern_matched = if starts_with.is_some() {
                                    starts_with.as_ref().unwrap().clone()
                                } else if ends_with.is_some() {
                                    ends_with.as_ref().unwrap().clone()
                                } else {
                                    "random".to_string()
                                };

                                let result = VanityResult {
                                    public_key: pubkey.to_string(),
                                    private_key: bs58::encode(private_key_bytes).into_string(),
                                    pattern_matched,
                                    attempts: local_attempts,
                                    found_at: chrono::Utc::now(),
                                };

                                if tx_clone.send(result).is_err() {
                                    return; // Channel closed, exit thread
                                }
                            }
                        }
                    }
                }

                // Update global attempt counter
                {
                    let mut global_attempts = total_attempts.lock().unwrap();
                    *global_attempts += batch_size as u64;
                }
            }
        });

        // Also spawn CPU workers for additional parallelization
        let cpu_handles = self.spawn_cpu_workers(&tx, target_count)?;

        // Collect results
        let mut found_count = 0;
        while found_count < target_count {
            if let Ok(result) = rx.recv() {
                {
                    let mut results = self.results.lock().unwrap();
                    results.push(result);
                }
                found_count += 1;
                progress_bar.inc(1);

                // Save immediately
                self.save_results()?;
            }
        }

        // Clean up threads
        drop(tx);
        let _ = gpu_handle.join();
        for handle in cpu_handles {
            let _ = handle.join();
        }

        Ok(())
    }

    fn spawn_cpu_workers(
        &self,
        tx: &Sender<VanityResult>,
        target_count: usize,
    ) -> Result<Vec<thread::JoinHandle<()>>> {
        let num_cpu_threads = (num_cpus::get() / 2).max(1); // Use half CPU cores for GPU mode
        let mut handles = Vec::new();

        for _ in 0..num_cpu_threads {
            let tx = tx.clone();
            let starts_with = self.starts_with.clone();
            let ends_with = self.ends_with.clone();
            let case_sensitive = self.case_sensitive;
            let total_attempts = Arc::clone(&self.total_attempts);

            let handle = thread::spawn(move || {
                let mut local_attempts = 0u64;
                let mut rng = rand::thread_rng();
                let batch_size = 100_000; // Smaller batches for CPU workers

                loop {
                    let seeds: Vec<u32> = (0..batch_size).map(|_| rng.gen()).collect();

                    for &seed in &seeds {
                        local_attempts += 1;

                        let keypair = generate_keypair_from_seed(seed);
                        let pubkey = keypair.pubkey();

                        if check_pattern_match(&pubkey, &starts_with, &ends_with, case_sensitive) {
                            let pattern_matched = if starts_with.is_some() {
                                starts_with.as_ref().unwrap().clone()
                            } else if ends_with.is_some() {
                                ends_with.as_ref().unwrap().clone()
                            } else {
                                "random".to_string()
                            };

                            let result = VanityResult {
                                public_key: pubkey.to_string(),
                                private_key: bs58::encode(keypair.to_bytes()).into_string(),
                                pattern_matched,
                                attempts: local_attempts,
                                found_at: chrono::Utc::now(),
                            };

                            if tx.send(result).is_err() {
                                return; // Channel closed, exit thread
                            }
                        }
                    }

                    // Update global attempt counter
                    {
                        let mut global_attempts = total_attempts.lock().unwrap();
                        *global_attempts += batch_size as u64;
                    }
                }
            });

            handles.push(handle);
        }

        Ok(handles)
    }

    fn display_results(&self) {
        let results = self.results.lock().unwrap();
        println!("\n📋 Found {} vanity addresses:", results.len());
        println!("{}", "=".repeat(80));

        for (i, result) in results.iter().enumerate() {
            println!("{}. Public Key: {}", i + 1, result.public_key);
            println!("   Private Key: {}", result.private_key);
            println!("   Pattern: {}", result.pattern_matched);
            println!("   Attempts: {}", format_attempts(result.attempts));
            println!(
                "   Found: {}",
                result.found_at.format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!();
        }
    }

    fn save_results(&self) -> Result<()> {
        let results = self.results.lock().unwrap();
        save_results(&results, &self.output_path)
    }
}
