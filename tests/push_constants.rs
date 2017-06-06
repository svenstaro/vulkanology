//! This is an example of an elaborate shader test.

#[macro_use]
extern crate vulkano;
#[macro_use]
extern crate vulkanology;
extern crate rand;

use std::time::Duration;

/// This test shows how to use push constants in tests.
#[test]
fn test_with_push_constants() {
    const NUM_INVOCATIONS: usize = 640000;
    const A: f32 = 4.0;
    const B: f32 = 10.0;

    // Create the environment.
    pipeline!{
        shader_path: "tests/shaders/push_constants.comp",
        workgroup_count: [100, 100, 1],
        buffers: {
            result: [f32;NUM_INVOCATIONS]
        },
        push_constants: {
            a: f32 = A,
            b: f32 = B
        },
        execution_command: execute_shader
    };

    // Execute the shader
    execute_shader();

    // Assert the validity of the results.
    {
        // Get read references to the remote buffers.
        let result_buffer = result.read(Duration::new(1, 0)).unwrap();
        let result_buffer_iter = result_buffer.iter().enumerate();

        for (invocation_uid, remote_result) in result_buffer_iter {
            let local_result = A * invocation_uid as f32 + B;
            println!("{} {}", remote_result, local_result);
            assert!((local_result - remote_result).abs() < 0.0001);
        }
    }
}
