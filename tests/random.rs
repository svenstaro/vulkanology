//! This crate provides macros for writing simple vulkan compute shader tests
//! using the tomaka/vulkano library.

extern crate vulkano;
#[macro_use]
extern crate vulkanology;
extern crate rand;

use std::time::Duration;
use rand::{Rng, StdRng};

use vulkano::buffer::cpu_access::{WriteLock, ReadLock};

/// Implementation of the xoroshiro128+ PRNG.
/// For reference see: http://xoroshiro.di.unimi.it/xoroshiro128plus.c
fn xoroshiro128plus(seed: &mut [u64; 2]) -> u64 {
    let s0 = seed[0];
    let mut s1 = seed[1];
    let result = s0.wrapping_add(s1);

    s1 ^= s0;
    seed[0] = s0.rotate_left(55) ^ s1 ^ (s1 << 14);
    seed[1] = s1.rotate_left(36);

    result
}

#[test]
fn test_random_next_u64() {
    const NUM_INVOCATIONS: usize = 640000;
    const PRNG_XOROSHIRO128PLUS_NUM_U64: usize = 2;

    // Create the environment.
    pipeline!{
        workgroup_count: [100, 100, 1],
        buffers: {
            prng: [u64;NUM_INVOCATIONS*PRNG_XOROSHIRO128PLUS_NUM_U64],
            result: [u64;NUM_INVOCATIONS]
        },
        execution_command: execute_shader
    };

    // Fill buffers with random seeds.
    let mut seed_generator = StdRng::new().unwrap();
    let mut seed_generator_clone = seed_generator.clone();
    {
        let mut mapping: WriteLock<[u64]> = prng.write(Duration::new(1, 0)).unwrap();
        for item in mapping.iter_mut() {
            *item = seed_generator.next_u64();
        }
    }

    // Execute the shader
    execute_shader();

    // Assert the validity of the results.
    {
        let seed_buffer = prng.read(Duration::new(1, 0)).unwrap();
        let result_buffer = result.read(Duration::new(1, 0)).unwrap();
        for invocation_uid in 0..NUM_INVOCATIONS {
            let mut local_seed = [seed_generator_clone.next_u64(), seed_generator_clone.next_u64()];
            let local_result = xoroshiro128plus(&mut local_seed);

            let seed_id_offset = PRNG_XOROSHIRO128PLUS_NUM_U64 * invocation_uid;
            assert_eq!(seed_buffer[seed_id_offset], local_seed[0]);
            assert_eq!(seed_buffer[seed_id_offset + 1], local_seed[1]);
            assert_eq!(result_buffer[invocation_uid], local_result);
        }
    }
}
