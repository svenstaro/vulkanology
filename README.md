# vulkanology
Test Vulkan compute shaders using Rust

[![Docs Status](https://docs.rs/vulkanology/badge.svg)](https://docs.rs/vulkanology)
[![license](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/svenstaro/vulkanology/blob/master/LICENSE)
[![Crates.io](https://img.shields.io/crates/v/vulkanology.svg)](https://crates.io/crates/vulkanology)
[![Crates.io](https://img.shields.io/crates/d/vulkanology.svg)](https://crates.io/crates/vulkanology)

This crate aims at providing a very simple interface for writing tests for vulkan GLSL shader programs.

# Example

```rust
extern crate vulkano;
#[macro_use]
extern crate vulkanology;
extern crate rand;

#[allow(unused_variables)]
fn main() {
    // The total number of invocations of your shader is defined in two places:
    //      - The workgroup_count, which is defined in the pipeline macro.
    //      - The workgroup_size which is defined in the shader program header.

    // Here we compute the total number of invocations. The workgroup size is 8x8x1,
    // and the workgroup count will be 100x100x1.
    let total_num_invocations = (8 * 8) * (100 * 100);

    // I. Invoke the `pipeline!` macro.
    // The macro parameters are:
    //    1. The path to the shader program, relative to the crate root.
    //        `shader_path: "path/to/shader/program.comp"`
    //    2. A three-dimensional array defining the workgroup count:
    //        `workgroup_count: [1, 2, 3],`
    //    3. The buffers that your test shader uses:
    //        `buffers: { input_data: [u32;4], some_buffer: [Dennis;42] },`
    //    4. The name of the shader execution:
    //        `execution_command: run_example_shader_function_name`
    pipeline!{
        shader_path: "tests/shaders/example.comp",
        workgroup_count: [100, 100, 1],
        buffers: {
           data: [u32; total_num_invocations],
           result: [u32; total_num_invocations]
        },
        execution_command: execute_shader
    }

    // II. Fill your buffers with input data. The buffers are bound to the
    //      names given in the `pipeline!` macro.
    {
        use std::time::Duration;
        use rand::random;

        use vulkano::buffer::cpu_access::WriteLock;
        let mut mapping: WriteLock<[u32]> = data.write(Duration::new(1, 0)).unwrap();

        for item in mapping.iter_mut() {
            *item = random::<u32>();
        }
    }

    // III. Execute the shader.
    //    `run_example_shader_function_name();`
    execute_shader();

    // IV. Assert validity of the results.
    //    `assert!(datainbuffersisvalid())`
    {
        use std::time::Duration;
        use vulkano::buffer::cpu_access::ReadLock;
        let input: ReadLock<[u32]> = data.read(Duration::new(1, 0)).unwrap();
        let output: ReadLock<[u32]> = result.read(Duration::new(1, 0)).unwrap();
        let zipped = input.iter().zip(output.iter());

        for (invocation_uid, (item_in, item_out)) in zipped.enumerate() {
            assert_eq!(*item_out, (*item_in).wrapping_mul(invocation_uid as u32));
        }
    }
}
```
