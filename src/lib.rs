//! This crate provides macros for writing simple vulkan compute shader tests
//! using the [tomaka/vulkano library](https://github.com/tomaka/vulkano).
//!
//! ## About
//!
//! A core problem of developing shaders is the rather difficult environment in which they are
//! executed. Even simple things can go wrong and cost the developer a lot of time to fix.
//! This crate aims at providing a simple-to-use environment for writing vulkan compute
//! shader tests. It uses the vulkano rust-vulkan bindings end exports macros for a fast
//! implementation of tests. These macros mostly generate vulkano boilerplate instantiation code.
//! The interface to the shader are CPU accessible buffers which you can read and write at will
//! and a function for executing the shader code and waiting for the result.
//!
//!
//! ## Import
//!
//! This library does not have any dependencies as it just exports macros for use in your
//! testing modules. This is also required to prevent version incompatibilities between
//! a vulkano which would be used here and your project-local vulkano.
//!
//! Due to the use of utility function and macros from the vulkano crate
//! (which you don't need to access, unless you want to) you need to use the
//! following crates in your application header:
//!
//! ```
//! #[macro_use]
//! extern crate vulkano;
//! #[macro_use]
//! extern crate vulkanology;
//! #
//! # fn main() {}
//! ```
//!
//! For basic usage of the library you can refer to the doc-tests and `tests/shaders/example.comp`.
//! For a working example of a fairly elaborate shader test please refer to: `tests/random.rs`
//! and `tests/shaders/random.comp`.
//!
#![deny(missing_docs)]
#![feature(macro_reexport)]

/// Creates a `vulkano::Instance`. Does not enable any instance extensions.
///
/// # Panics
///
/// Panics if the vulkano instance loading procedure fails.
///
/// # Example
///
/// ```
/// # // These tests du not require vulkano-macros,
/// # // therefore the `macro_use` will be omitted here, unless required.
/// # extern crate vulkano;
/// # #[macro_use]
/// # extern crate vulkanology;
/// #
/// # #[allow(unused_variables)]
/// # fn main() {
/// // Simply invoke the macro and assign the result.
/// let instance = instance!();
/// # }
/// ```
#[macro_export]
macro_rules! instance {
    () => ({
        use vulkano::instance::{Instance, InstanceExtensions};
        let extensions = &InstanceExtensions::none();
        Instance::new(None, extensions, None).expect("Failed to initialize vulkano.")
    })
}

/// This macro generates code for loading a `PhysicalDevice`. It takes
/// the instance variable name and an optional list of features which the device
/// should support.
/// All available features are defined here:
/// https://github.com/tomaka/vulkano/blob/master/vulkano/src/features.rs
///
/// # Panics
///
/// Panics if no device matching the requirements has been found.
///
/// # Example
///
/// ```
/// # extern crate vulkano;
/// # #[macro_use]
/// # extern crate vulkanology;
/// #
/// # #[allow(unused_variables)]
/// # fn main() {
/// // First initialize a `vulkano::Instance`.
/// let instance = instance!();
///
/// // Select the first physical device which supports compute shaders.
/// {
///     // With no explicitly required features:
///     let physical_device = physical_device!(instance);
/// }
/// {
///     // With some features:
///     let physical_device = physical_device!(
///         instance,
///         robust_buffer_access,
///         full_draw_index_uint32);
/// }
/// # }
/// ```
#[macro_export]
macro_rules! physical_device {
    // Rule for selecting a device with specific features.
    ($instance:ident, $($feature:ident),+) => ({
        use vulkano::instance::{PhysicalDevice};
        PhysicalDevice::enumerate(&$instance).find(|p| {
            let supported_features = p.supported_features();
            true $( && supported_features.$feature )*
        }).expect("No physical devices are available.")
    });

    // Rule for selecting the first available physical
    // device when no features are required.
    ($instance:ident) => ({
        use vulkano::instance::{PhysicalDevice};
        PhysicalDevice::enumerate(&$instance).next()
            .expect("No physical devices are available.")
    })
}

/// Creates a `Device` and a `Queue` for compute operations.
///
/// # Panics
///
/// Panics if no conpute-compatible queue has been found, or the
/// device could not be initialized.
///
/// # Example
///
/// ```
/// # extern crate vulkano;
/// # #[macro_use]
/// # extern crate vulkanology;
/// #
/// # #[allow(unused_variables)]
/// # fn main() {
/// let instance = instance!();
/// let physical_device = physical_device!(instance);
/// let (device, queue) = device_and_queue!(physical_device);
/// # }
/// ```
#[macro_export]
macro_rules! device_and_queue {
    ($physical_device:ident) => ({
        use vulkano::device::{Device, DeviceExtensions};

        // Select a queue family which supports compute operations.
        let mut queue_families = $physical_device.queue_families();
        let queue_family = queue_families.find(|q| q.supports_compute())
            .expect("Couldn't find a compute queue family.");

        // Initialize a device and a queue.
        let device_extensions = DeviceExtensions::none();
        let (device, mut queues) = Device::new(&$physical_device,
                                               &$physical_device.supported_features(),
                                               &device_extensions,
                                               [(queue_family, 0.5)].iter().cloned())
            .expect("Failed to create device.");

        // We only requested one queue, so `queues` is an array with only one element.
        (device, queues.next().unwrap())
    })
}

/// Creates a new uninitialized buffer of type `$buf_type` of length `$buf_len`.
///
/// # Panics
///
/// If the array fails to be initialized.
///
/// # Examples
///
/// ```
/// # extern crate vulkano;
/// # #[macro_use]
/// # extern crate vulkanology;
/// #
/// # #[allow(unused_variables)]
/// # fn main() {
/// let instance = instance!();
/// let physical_device = physical_device!(instance);
/// let (ref device, ref queue) = device_and_queue!(physical_device);
///
/// // Initialize a buffer.
/// let buffer = cpu_array_buffer!(device, queue, u32, 13*31);
/// # }
/// ```
#[macro_export]
macro_rules! cpu_array_buffer {
    ($device:ident, $queue:ident, $buf_type:ty, $buf_len:expr) => ({
        use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
        unsafe {
            CpuAccessibleBuffer::<[$buf_type]>::uninitialized_array(
                $device,
                $buf_len,
                &BufferUsage::all(),
                Some($queue.family()))
                .expect("Failed to create a cpu accessible buffer.")
        }
    })
}

/// This macro is the core of the shader-testing framework.
/// It generates code for initializing the vulkano environment,
/// it allocates CPU accessible buffers, it compiles the shader,
/// it sets up a `ComputePipeline` and provides a function
/// for executing the shader.
///
/// # Panics
///
/// * If the instance, physical_device, device or queue cannot be selected/initialized.
/// * If the buffers cannot be initialized.
/// * If the shader cannot be loaded.
/// * If the pipeline cannot be created.
///
/// # Example
///
/// ```
/// # #[macro_use]
/// # extern crate vulkano;
/// # #[macro_use]
/// # extern crate vulkanology;
/// # extern crate rand;
/// #
/// # #[allow(unused_variables)]
/// # fn main() {
/// // The total number of invocations of your shader is defined in two places:
/// //      - The workgroup_count, which is defined in the pipeline macro.
/// //      - The workgroup_size which is defined in the shader program header.
///
/// // Here we compute the total number of invocations. The workgroup size is 8x8x1,
/// // and the workgroup count will be 100x100x1.
/// let total_num_invocations = (8 * 8) * (100 * 100);
///
/// // I. Invoke the `pipeline!` macro.
/// // The macro parameters are:
/// //    1. The path to the shader program, relative to the crate root.
/// //        `shader_path: "path/to/shader/program.comp"`
/// //    2. A three-dimensional array defining the workgroup count:
/// //        `workgroup_count: [1, 2, 3],`
/// //    3. The buffers that your test shader uses:
/// //        `buffers: { input_data: [u32;4], some_buffer: [Dennis;42] },`
/// //    4. The name of the shader execution:
/// //        `execution_command: run_example_shader_function_name`
/// pipeline!{
///     shader_path: "tests/shaders/example.comp",
///     workgroup_count: [100, 100, 1],
///     buffers: {
///        data: [u32; total_num_invocations],
///        result: [u32; total_num_invocations]
///     },
///     execution_command: execute_shader
/// }
///
/// // II. Fill your buffers with input data. The buffers are bound to the
/// //      names given in the `pipeline!` macro.
/// {
///     use std::time::Duration;
///     use rand::random;
///
///     use vulkano::buffer::cpu_access::WriteLock;
///     let mut mapping: WriteLock<[u32]> = data.write(Duration::new(1, 0)).unwrap();
///
///     for item in mapping.iter_mut() {
///         *item = random::<u32>();
///     }
/// }
///
/// // III. Execute the shader.
/// //    `run_example_shader_function_name();`
/// execute_shader();
///
/// // IV. Assert validity of the results.
/// //    `assert!(datainbuffersisvalid())`
/// {
///     use std::time::Duration;
///     use vulkano::buffer::cpu_access::ReadLock;
///     let input: ReadLock<[u32]> = data.read(Duration::new(1, 0)).unwrap();
///     let output: ReadLock<[u32]> = result.read(Duration::new(1, 0)).unwrap();
///     let zipped = input.iter().zip(output.iter());
///
///     for (invocation_uid, (item_in, item_out)) in zipped.enumerate() {
///         assert_eq!(*item_out, (*item_in).wrapping_mul(invocation_uid as u32));
///     }
/// }
/// # }
/// ```
///
#[macro_export]
macro_rules! pipeline {
    {
        shader_path: $shader_path:expr,
        workgroup_count: [$workgroup_x:expr, $workgroup_y:expr, $workgroup_z:expr],
        buffers: { $( $buf_ident:ident : [$buf_type:ty;$buf_len:expr] ),* },
        execution_command: $exec_cmd:ident
    } => {
        use vulkano::command_buffer::PrimaryCommandBufferBuilder;
        use vulkano::command_buffer::submit as submit_command;
        use vulkano::descriptor::descriptor_set::DescriptorPool;
        use vulkano::pipeline::ComputePipeline;

        // Include the shader wrapper.
        mod shader {
            #![allow(dead_code)]
            include!{concat!(env!("OUT_DIR"), concat!("/shaders/", $shader_path))}
        }

        // Create the pipeline layout wrapper.
        mod layout_definition {
            pipeline_layout!{
                buffers: {
                    $( $buf_ident: StorageBuffer<[$buf_type]> ),*
                }
            }
        }

        // Init vulkano.
        let instance = instance!();
        let physical_device = physical_device!(instance);
        let (ref device, ref queue) = device_and_queue!(physical_device);

        // Allocate buffers.
        $( let $buf_ident = cpu_array_buffer!(device, queue, $buf_type, $buf_len); )*

            // Create descriptor pool.
            let descriptor_pool = DescriptorPool::new(device);

        // Create pipeline layout.
        let pipeline_layout = layout_definition::CustomPipeline::new(device).unwrap();
        let buffer_descriptors = layout_definition::buffers::Descriptors {
            $( $buf_ident: &$buf_ident, )*
        };
        let buffer_set = layout_definition::buffers::Set::new(&descriptor_pool,
                                                              &pipeline_layout,
                                                              &buffer_descriptors);

        // Load the shader and assemble the pipeline.
        let compute_shader = shader::Shader::load(device).expect("Failed to create shader module.");
        let pipeline = ComputePipeline::new(device,
                                            &pipeline_layout,
                                            &compute_shader.main_entry_point(),
                                            &())
            .expect("Failed to create compute pipeline.");

        // Assemble and return the execution command.
        let workgroup_count = [$workgroup_x, $workgroup_y, $workgroup_z];
        let execution_command = PrimaryCommandBufferBuilder::new(device, queue.family())
            .dispatch(&pipeline, buffer_set, workgroup_count, &())
            .build();
        let $exec_cmd = || {
            submit_command(&execution_command, queue).unwrap();
        };
    }
}
