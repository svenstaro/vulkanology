extern crate vulkano_shaders;

use vulkano_shaders::ShaderType;

fn main() {
    let shader_list = [
        ("tests/shaders/example.comp", ShaderType::Compute),
        ("tests/shaders/push_constants.comp", ShaderType::Compute),
        ("tests/shaders/random.comp", ShaderType::Compute)
    ];
    vulkano_shaders::build_glsl_shaders(shader_list.iter().cloned());
}
