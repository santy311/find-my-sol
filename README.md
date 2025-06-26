# Solana Vanity Address Generator

An extremely fast Rust-based Solana vanity address generator with OpenCL multi-GPU support. This tool can generate Solana keypairs that match specific patterns in their public key addresses.

## Features

- ⚡ **Extremely Fast**: Multi-threaded CPU and GPU acceleration
- 🎯 **Pattern Matching**: Search for addresses that start with or end with specific patterns
- 🔧 **OpenCL Support**: Multi-GPU acceleration for maximum performance
- 💾 **Persistent Storage**: Automatically saves results to JSON files
- 📊 **Progress Tracking**: Real-time progress bars and statistics
- 🎛️ **Flexible Options**: Case-sensitive matching, custom iteration bits, device selection

## Installation

### Prerequisites

- Rust (latest stable version)
- OpenCL drivers and SDK (for GPU acceleration)
- macOS, Linux, or Windows

### Build

```bash
# Clone the repository
git clone <repository-url>
cd vanity

# Build the project
cargo build --release

# Install globally (optional)
cargo install --path .
```

## Usage

### Show Available OpenCL Devices

First, check what GPU devices are available for acceleration:

```bash
cargo run -- show-devices
```

This will display all available OpenCL devices with their specifications.

### Search for Vanity Addresses

#### Basic Usage

```bash
# Search for addresses starting with "ABC"
cargo run -- search-pubkey --starts-with ABC

# Search for addresses ending with "XYZ"
cargo run -- search-pubkey --ends-with XYZ

# Search for addresses starting with "ABC" and ending with "XYZ"
cargo run -- search-pubkey --starts-with ABC --ends-with XYZ
```

#### Advanced Options

```bash
# Generate 5 addresses starting with "SOL"
cargo run -- search-pubkey --starts-with SOL --count 5

# Use GPU device 0 for acceleration
cargo run -- search-pubkey --starts-with SOL --device 0

# Case-sensitive matching
cargo run -- search-pubkey --starts-with SOL --case-sensitive

# Custom output file
cargo run -- search-pubkey --starts-with SOL --output my_results.json

# Higher iteration bits for more parallel work (default: 20)
cargo run -- search-pubkey --starts-with SOL --iteration-bits 24
```

#### Complete Example

```bash
# Generate 10 addresses starting with "SOL" using GPU device 0
cargo run -- search-pubkey \
  --starts-with SOL \
  --count 10 \
  --device 0 \
  --case-sensitive \
  --output sol_addresses.json \
  --iteration-bits 22
```

## Command Line Options

### `search-pubkey` Command

| Option             | Short | Description                                                       | Default             |
| ------------------ | ----- | ----------------------------------------------------------------- | ------------------- |
| `--starts-with`    | `-s`  | Pattern that the public key should start with                     | None                |
| `--ends-with`      | `-e`  | Pattern that the public key should end with                       | None                |
| `--count`          | `-c`  | Number of vanity addresses to generate                            | 1                   |
| `--device`         | `-d`  | OpenCL device index to use                                        | CPU-only            |
| `--iteration-bits` |       | Number of bits to use for iteration (higher = more parallel work) | 20                  |
| `--case-sensitive` | `-C`  | Case sensitive matching                                           | false               |
| `--output`         | `-o`  | Output file to save results                                       | vanity_results.json |

### `show-devices` Command

Lists all available OpenCL devices with their specifications.

## Performance Tips

1. **Use GPU Acceleration**: Select a GPU device for significantly faster generation
2. **Higher Iteration Bits**: Increase `--iteration-bits` for more parallel work (20-24 recommended)
3. **Shorter Patterns**: Shorter patterns are much faster to find
4. **Case Insensitive**: Use case-insensitive matching when possible for better performance

## Output Format

Results are saved in JSON format:

```json
[
  {
    "public_key": "SOL1234567890abcdef...",
    "private_key": "4xQy...",
    "pattern_matched": "SOL",
    "attempts": 1234567,
    "found_at": "2024-01-01T12:00:00Z"
  }
]
```

## Performance Benchmarks

- **CPU-only**: ~100K-500K attempts/second (depending on CPU)
- **GPU acceleration**: ~1M-10M attempts/second (depending on GPU)
- **Multi-GPU**: Scales linearly with number of GPUs

## Technical Details

### Architecture

- **Multi-threading**: Uses all available CPU cores with Rayon
- **GPU Acceleration**: OpenCL kernels for parallel keypair generation
- **Hybrid Mode**: Combines CPU and GPU for maximum throughput
- **Memory Efficient**: Streams results to disk immediately

### Pattern Matching

- Supports both prefix and suffix matching
- Case-sensitive and case-insensitive modes
- Base58 encoding for Solana addresses
- Real-time probability estimation

### Security

- Uses cryptographically secure random number generation
- Ed25519 keypair generation following Solana standards
- Private keys are properly encoded in Base58

## Troubleshooting

### OpenCL Issues

If you encounter OpenCL errors:

1. Ensure OpenCL drivers are installed
2. Check device compatibility with `show-devices`
3. Try CPU-only mode if GPU acceleration fails

### Performance Issues

1. Use `--device` to select a specific GPU
2. Increase `--iteration-bits` for more parallel work
3. Ensure sufficient system memory
4. Close other GPU-intensive applications

### Pattern Not Found

1. Check pattern length - longer patterns take exponentially more time
2. Verify case sensitivity settings
3. Use shorter patterns for faster results

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.
