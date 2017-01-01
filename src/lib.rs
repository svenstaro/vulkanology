//! This crate provides macros for writing simple vulkan compute shader tests
//! using the tomaka/vulkano library.
#![feature(macro_reexport)]

#[macro_use]
#[macro_reexport(pipeline_layout)]
extern crate vulkano;

/// Creates a `vulkano::Instance`. Does not enable any instance extensions.
///
/// # Panics
///
/// Panics if the vulkano instance loading procedure fails.
///
/// # Example
///
/// ```
/// # extern crate vulkano;
/// # #[macro_use]
/// # extern crate vulkanology;
///
/// # #[allow(unused_variables)]
/// # #[ignore]
/// # fn main() {
/// // Simply invoke the macro and assign the result.
/// let instance = instance!();
/// # }
/// ```
#[macro_export]
macro_rules! instance {
    () => ({
        use vulkano::instance::{Instance, InstanceExtensions};
        let ref extensions = InstanceExtensions::none();
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
///
/// # #[allow(unused_variables)]
/// # #[ignore]
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
///     let physical_device = physical_device!(instance, shader_int64, sparse_binding);
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
#[macro_export]
macro_rules! device_and_queue {
    ($physical_device:ident) => ({
        use vulkano::device::{Device, DeviceExtensions};

        // Select a queue family which supports graphics.
        let mut queue_families = $physical_device.queue_families();
        let queue_family = queue_families.find(|q| q.supports_compute())
            .expect("Couldn't find a graphical queue family.");

        // Initialize a device and a queue.
        let device_extensions = DeviceExtensions::none();
        let (device, mut queues) = Device::new(&$physical_device,
                                               &$physical_device.supported_features(),
                                               &device_extensions,
                                               [(queue_family, 0.5)].iter().cloned())
            .expect("Failed to create device.");

        (device, queues.next().unwrap())
    })
}

/// This macro is the core of the shader-testing framework.
/// It generates code for initializing the vulkano environment,
/// it allocates CPU-side buffers, it compiles the shader,
/// it sets up a `ComputePipeline` and provides a function
/// for executing the shader.
///
/// # Examples
///
/// 1. Invoke the `pipeline!` macro.
///   * The macro parameters are:
///     1. A three-dimensional array defining the workgroup count:
///         `workgroup_count: [100, 100, 1],`
///     2. The buffers that your test shader uses:
///         `buffers: { a: [u32;4], b: [Dennis;42] },`
///     3. The name of the shader execution:
///         `execution_command: execute_shader_function_name_table_mouse`
/// 2. Fill your buffers with input data.
/// 3. Execute the shader.
///     `execute_shader_function_name_table_mouse();`
///
/// 4. Assert validity of the results.
///     `assert!(/*datainbuffersisvalid*/ true)`
///
#[macro_export]
macro_rules! pipeline {
    {
        workgroup_count: [$workgroup_x:expr, $workgroup_y:expr, $workgroup_z:expr],
        buffers: {
            $( $buf_ident:ident : [$buf_type:ty;$buf_len:expr] ),*
        },
        execution_command: $exec_cmd:ident
    } => {
        use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
        use vulkano::command_buffer::PrimaryCommandBufferBuilder;
        use vulkano::command_buffer::submit as submit_command;
        use vulkano::descriptor::descriptor_set::DescriptorPool;
        use vulkano::pipeline::ComputePipeline;

        // Include the shader wrapper.
        mod shader {
            #![allow(dead_code)]
            include!{concat!(env!("OUT_DIR"), "/shaders/tests/shaders/random.comp")}
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
        $( let $buf_ident = unsafe {
            CpuAccessibleBuffer::<[$buf_type]>::uninitialized_array(
                       device,
                       $buf_len,
                       &BufferUsage::all(),
                       Some(queue.family()))
                   .expect("Failed to create cpu accessible buffer.")
        }; )*

        // Create descriptor pool.
        let descriptor_pool = DescriptorPool::new(device);

        // Create pipeline.
        let pipeline_layout = layout_definition::CustomPipeline::new(device).unwrap();
        let buffer_descriptors = layout_definition::buffers::Descriptors {
            $( $buf_ident: &$buf_ident, )*
        };
        let buffer_set = layout_definition::buffers::Set::new(&descriptor_pool,
                                                              &pipeline_layout,
                                                              &buffer_descriptors);

        let compute_shader = shader::Shader::load(device).expect("Failed to create shader module.");
        let pipeline = ComputePipeline::new(device,
                                            &pipeline_layout,
                                            &compute_shader.main_entry_point(),
                                            &())
            .expect("Failed to create compute pipeline.");

        let workgroup_count = [$workgroup_x, $workgroup_y, $workgroup_z];
        let execution_command = PrimaryCommandBufferBuilder::new(device, queue.family())
            .dispatch(&pipeline, buffer_set, workgroup_count, &())
            .build();

        let $exec_cmd = || {
            submit_command(&execution_command, queue).unwrap();
        };
    }
}
