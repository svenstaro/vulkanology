//! This module exports shader building tools which simplify the shader test building process.

use std::path::Path;
use std::fs::create_dir_all;

/// Concatenates GLSL source files inserting `#line` statements where necessary.
///
/// # Motivation
///
/// To reuse/reduce duplicate code in shaders and to be able to test certain code parts of shader
/// programs without updating both the actual shaders and the test shaders, one can split the
/// shaders into semantically grouped parts, so-called segments. Prior to building the shaders the
/// build script will have to concatenate the files. Without additional measures the errors of the
/// shader compiler would point to the generated files/lines. Therefore we insert the `#line`
/// macros which set the correct file name and line number.
///
/// # Panics
///
/// * If no files were given.
/// * If the target directory cannot be created.
/// * It a file cannot be opened.
/// * Some other file I/O operations fail.
///
/// # Example
///
/// ```
/// use std::fs::{create_dir_all, File};
/// use std::io::{BufRead, BufReader};
/// use std::path::Path;
/// use vulkanology::build_utils::concatenate_files;
///
/// // Inputs
/// let target = Path::new("target");
/// let path_a = target.join("a.in").to_path_buf();
/// let path_b = target.join("b.in").to_path_buf();
/// create_dir_all(target).unwrap();
///
/// // Outputs
/// let path_c = target.join("c.out");
/// 
/// // Concatenate the files.
/// concatenate_files(&[path_a, path_b], &path_c);
///
/// // Check whether the resulting file contains both of the input files and a `#line` pragma.
/// let input_files = [path_a, path_b];
/// let mut file_c = File::open(path_c).unwrap();
/// let mut lines_c = BufReader::new(&file_c);
/// 
/// fn contains_file(a: &mut BufReader<&File>, b: &mut BufReader<&File>) {
///     for (line_a, line_b) in a.lines().zip(b.lines()) {
///         assert_eq!(line_a.unwrap(), line_b.unwrap());
///     }
/// }
///
/// for input_file in &input_files {
///     let mut file_input = File::open(input_file).unwrap();
///     let mut lines_input = BufReader::new(&file_input);
///
///     contains_file(&mut lines_c, &mut lines_input);
/// }
/// 
/// ```
pub fn concatenate_files<P: AsRef<Path>>(file_names: &[P], write_to: &Path) {
    if file_names.len() == 0 {
        panic!("There must be at least one file to concatenate.");
    }

    use std::io::{Read, Write};
    use std::fs::File;

    // Each segment (.comp file) content is prefixed with the #line pragma. This makes the error
    // messages be displayed with their filename and actual line number instead of the position of
    // a line in the concatenated file.
    let line_pragma = b"#line 1 \"";
    let quotes = b"\"\n";

    let target_dir = write_to.parent().unwrap();
    create_dir_all(target_dir).expect("Failed to create target directory.");
    let mut file_out = File::create(write_to)
        .expect(format!("Failed to open output file: {}", write_to.display()).as_ref());
    let mut file_names_iter = file_names.iter();

    fn append_file(file_out: &mut File, file_name: &Path) {
        let mut file_in = match File::open(file_name) {
            Ok(file) => file,
            Err(err) => {
                panic!("Failed to open input file: {}\n{}",
                       file_name.display(),
                       err)
            }
        };

        // Rerun the build script if one of the files changed
        // for reference see: http://doc.crates.io/build-script.html#outputs-of-the-build-script
        println!("cargo:rerun-if-changed={}", file_name.display());
        let mut buffer = Vec::new();
        file_in.read_to_end(&mut buffer).expect("Failed to read from file.");
        file_out.write_all(&buffer).expect("Failed to write to file.");
    }

    // Copy the first file without any preceeding pragmas
    let first_file = file_names_iter.next().unwrap();
    append_file(&mut file_out, first_file.as_ref());

    for file_name in file_names_iter {
        let file_name_path = file_name.as_ref();
        let file_name_bytes = file_name_path.to_str().unwrap().as_bytes();
        file_out.write_all(line_pragma).expect("Failed to insert line pragma.");
        file_out.write_all(file_name_bytes).expect("Failed to write file name.");
        file_out.write_all(quotes).expect("Failed to write closing quotes.");
        append_file(&mut file_out, file_name_path);
    }
}

/// A simple macro for generating the code which compiles test shaders.
/// * `$group_prefix` the prefix of the directory where tests for a particular UUT are located.
/// * `$shader_name` the name of the shader test (should be a unique name).
/// * `$segment` a list of shader segments to include between the header and the main.
///
/// The directory structure for a single shader test looks like the following:
/// The files you have to provide are:
/// `tests/something.rs` a regular rust integration test.
/// `tests/shaders/<$group_prefix>/<$shader_name>_header.comp` - The test shader header.
/// `tests/shaders/<$group_prefix>/<$shader_name>_main.comp` - The test shader main.
/// `src/shaders/whatever.comp` - Some segments which the test shader includes and tests.
///
/// The GLSL files are then concatenated into a complete shader:
/// `target/test_shaders/<$shader_name>.comp`
///
/// Finally these shaders are compiled and wrapped into Rust code which is located at:
/// `<OUT_DIR>/shaders/target/test_shaders/<$shader_name>.comp`
///
///
/// # Example
///
/// ```
/// // The path to the normal shader segments.
/// let src_shaders = Path::new("src/shaders");
/// let random = src_shaders.join("random.comp");
///
/// // Tests for the random segment.
/// let random_shaders = Path::new("random");
///
/// // next_u64().
/// gen_simple_test_shader!{
///     group_prefix: random_shaders,
///     shader_name: test_random_next_u64,
///     segments: [random.clone()]
/// }
/// ```
macro_rules! gen_simple_test_shader {
    (
        group_prefix: $group_prefix:ident,
        shader_name: $shader_name:ident,
        segments: [ $( $segment:expr ),* ]
    ) => {
        let path_and_group = Path::new("tests/shaders").join($group_prefix);
        let segments = [
            path_and_group.join(concat!(stringify!($shader_name), "_header.comp")),
            $( $segment, )*
            path_and_group.join(concat!(stringify!($shader_name), "_main.comp"))
        ];
        let output = Path::new("target/test_shaders")
            .join(concat!(stringify!($shader_name), ".comp"));
        concatenate_files(&segments, &output);
        let $shader_name = (output.to_str().unwrap(), ShaderType::Compute);
    }
}
