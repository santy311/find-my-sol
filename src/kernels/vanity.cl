// OpenCL kernel for Solana vanity address generation
// Optimized for maximum performance on modern GPUs

// SHA256 constants
constant uint k[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

// Optimized SHA256 functions using bit operations
uint rotr32(uint x, int n) {
    return (x >> n) | (x << (32 - n));
}

uint ch(uint x, uint y, uint z) {
    return (x & y) ^ (~x & z);
}

uint maj(uint x, uint y, uint z) {
    return (x & y) ^ (x & z) ^ (y & z);
}

uint sigma0(uint x) {
    return rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22);
}

uint sigma1(uint x) {
    return rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25);
}

uint gamma0(uint x) {
    return rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3);
}

uint gamma1(uint x) {
    return rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10);
}

// Use __private for local arrays in device functions
void sha256_transform_optimized(uint state[8], __private uint* w) {
    uint a, b, c, d, e, f, g, h;
    uint t1, t2;
    
    // Initialize working variables
    a = state[0];
    b = state[1];
    c = state[2];
    d = state[3];
    e = state[4];
    f = state[5];
    g = state[6];
    h = state[7];
    
    // Main loop - unrolled for better performance
    #pragma unroll
    for (int i = 0; i < 64; i++) {
        if (i >= 16) {
            w[i] = gamma1(w[i-2]) + w[i-7] + gamma0(w[i-15]) + w[i-16];
        }
        
        t1 = h + sigma1(e) + ch(e, f, g) + k[i] + w[i];
        t2 = sigma0(a) + maj(a, b, c);
        h = g;
        g = f;
        f = e;
        e = d + t1;
        d = c;
        c = b;
        b = a;
        a = t1 + t2;
    }
    
    // Add the working variables back into state
    state[0] += a;
    state[1] += b;
    state[2] += c;
    state[3] += d;
    state[4] += e;
    state[5] += f;
    state[6] += g;
    state[7] += h;
}

// Generate multiple high-quality seeds per work item
uint4 generate_seeds_batch(uint base_seed, uint work_item_id, uint offset) {
    __private uint w[64];
    
    // Initialize SHA256 state
    uint state[8] = {
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
    };
    
    // Create input data with multiple variations
    uint data[16] = {0};
    data[0] = base_seed;
    data[1] = work_item_id;
    data[2] = offset;
    data[3] = base_seed ^ 0x12345678;
    data[4] = work_item_id ^ 0x87654321;
    data[5] = offset ^ 0xdeadbeef;
    data[6] = base_seed ^ 0xbeefdead;
    data[7] = work_item_id ^ 0xfeedface;
    
    // Copy data to local memory
    #pragma unroll
    for (int i = 0; i < 16; i++) {
        w[i] = data[i];
    }
    
    // Perform SHA256 transform
    sha256_transform_optimized(state, w);
    
    // Generate 4 seeds from the hash
    uint4 seeds;
    seeds.x = state[0] ^ state[1];
    seeds.y = state[2] ^ state[3];
    seeds.z = state[4] ^ state[5];
    seeds.w = state[6] ^ state[7];
    
    return seeds;
}

// Main kernel optimized for maximum throughput
__kernel void generate_random_seeds_optimized(
    __global const uint* base_seeds,
    __global uint* output_seeds,
    uint num_seeds_per_base,
    uint total_seeds_needed
) {
    int gid = get_global_id(0);
    int group_id = get_group_id(0);
    
    // Generate multiple seeds per work item for better efficiency
    uint base_seed = base_seeds[group_id % 1024]; // Use modulo to avoid out of bounds
    uint work_item_id = gid;
    
    // Generate 4 seeds per work item
    uint4 seeds = generate_seeds_batch(base_seed, work_item_id, group_id);
    
    // Store seeds with bounds checking
    int output_idx = gid * 4;
    if (output_idx < total_seeds_needed) {
        output_seeds[output_idx] = seeds.x;
    }
    if (output_idx + 1 < total_seeds_needed) {
        output_seeds[output_idx + 1] = seeds.y;
    }
    if (output_idx + 2 < total_seeds_needed) {
        output_seeds[output_idx + 2] = seeds.z;
    }
    if (output_idx + 3 < total_seeds_needed) {
        output_seeds[output_idx + 3] = seeds.w;
    }
}

// Alternative kernel for even higher throughput with vectorization
__kernel void generate_random_seeds_vectorized(
    __global const uint4* base_seeds,
    __global uint4* output_seeds,
    uint num_seeds_per_base
) {
    int gid = get_global_id(0);
    uint4 base_seed = base_seeds[gid / num_seeds_per_base];
    uint work_item_id = gid;
    
    // Generate 4 seeds using vector operations
    uint4 seeds;
    seeds.x = generate_seeds_batch(base_seed.x, work_item_id, 0).x;
    seeds.y = generate_seeds_batch(base_seed.y, work_item_id, 1).x;
    seeds.z = generate_seeds_batch(base_seed.z, work_item_id, 2).x;
    seeds.w = generate_seeds_batch(base_seed.w, work_item_id, 3).x;
    
    output_seeds[gid] = seeds;
} 