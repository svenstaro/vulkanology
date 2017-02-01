//! This module exports shader building tools which simplify the shader test building process.

use std::path::Path;
use std::io::{Read, Write};
use std::fs::File;
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
/// pragma which sets the correct file name and line number in the error reporter.
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
/// extern crate vulkanology;
/// # extern crate rand;
/// #
/// # fn main() {
/// use std::fs::{create_dir_all, File};
/// use std::io::{BufRead, BufReader, Lines, Write};
/// use std::path::Path;
/// use vulkanology::build_utils::concatenate_files;
///
/// // Working directory.
/// let target = Path::new("target");
/// create_dir_all(target).unwrap();
///
/// // Input segment paths.
/// let path_a = target.join("a.in");
/// let path_b = target.join("b.in");
///
/// // Fill segments with some random data
/// # fn fill_segment_with_random_test_data<P: AsRef<Path>>(path: P) {
/// #     let mut file = File::create(path.as_ref()).unwrap();
/// #     fn append_random_line(data: &mut Vec<u8>) {
/// #         let line_length = rand::random::<u32>() % 100;
/// #         for _ in 0..line_length {
/// #             let char = 97 + rand::random::<u8>() % 26;
/// #             data.push(char);
/// #         }
/// #         // Push a newline.
/// #         data.push(10)
/// #     }
/// #     let mut buffer = Vec::new();
/// #     let num_lines = rand::random::<u32>() % 100;
/// #     for _ in 0..num_lines {
/// #         append_random_line(&mut buffer);
/// #     }
/// #     file.write_all(&buffer).unwrap();
/// # }
/// fill_segment_with_random_test_data(&path_a);
/// fill_segment_with_random_test_data(&path_b);
///
/// // Concatenated output path.
/// let path_c = target.join("c.out");
///
/// // Concatenate the files.
/// concatenate_files(&[&path_a, &path_b], &path_c);
/// #
/// # fn contains_lines<P: AsRef<Path>>(lines1: &mut Lines<BufReader<&File>>, path2: P) {
/// #     let file2 = File::open(path2).unwrap();
/// #     let lines2 = BufReader::new(&file2).lines();
/// #     for line2 in lines2 {
/// #         let line1 = lines1.next().unwrap();
/// #         assert_eq!(line1.unwrap(), line2.unwrap());
/// #     }
/// # }
///
/// // Check whether the resulting file contains both of the input files and a `#line` pragma.
/// let file_c = File::open(path_c).unwrap();
/// let mut lines_c = BufReader::new(&file_c).lines();
///
/// // The first file content is not preceeded by a `#line` pragma.
/// contains_lines(&mut lines_c, path_a);
/// // Then comes the `#line` pragma ...
/// let expected_pragma = String::from(r#"#line 1 "target/b.in""#);
/// let actual_pragma = lines_c.next().unwrap().unwrap();
/// assert_eq!(expected_pragma, actual_pragma);
/// // ... and the content of the second file.
/// contains_lines(&mut lines_c, path_b);
/// # }
/// ```
///
pub fn concatenate_files<PI, PO>(file_names: &[PI], write_to: PO)
    where PI: AsRef<Path>,
          PO: AsRef<Path>
{
    if file_names.len() == 0 {
        panic!("There must be at least one file to concatenate.");
    }

    // Each segment (.comp file) content is prefixed with the #line pragma. This makes the error
    // messages be displayed with their filename and actual line number instead of the position of
    // a line in the concatenated file.
    let line_pragma = b"#line 1 \"";
    let quotes = b"\"\n";

    let write_to = write_to.as_ref();
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

        // Rerun the build script if one of the files changed.
        // for reference see: http://doc.crates.io/build-script.html#outputs-of-the-build-script
        println!("cargo:rerun-if-changed={}", file_name.display());
        let mut buffer = Vec::new();
        file_in.read_to_end(&mut buffer).expect("Failed to read from file.");
        file_out.write_all(&buffer).expect("Failed to write to file.");
    }

    // Copy the first file without any preceeding pragmas.
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
/// `<$segment>` - Some segments which the test shader includes and tests.
///
/// The GLSL files are then concatenated into a complete shader:
/// `target/test_shaders/<$shader_name>.comp`
///
/// Afterwards, the concatenated compute shader can either be directly passed to the Vulkan
/// API as GLSL code, or compiled into a Rust interface module by the [`vulkano_shaders`] crate.
/// [`vulkano_shaders`] provides a method called [`build_glsl_shaders`] which should be used
/// in the build script, right after concatenating the shader fragments. The generated module
/// would be located in:
/// `<OUT_DIR>/shaders/target/test_shaders/<$shader_name>.comp`
///
/// Fox examples on how to use this function see [this example].
///
/// # Example
///
/// ```
/// #[macro_use]
/// extern crate vulkanology;
///
/// # #[allow(unused_variables)]
/// fn main() {
///     // The path to the normal shader segments.
///     // In this example it's the same folder as the test shader segments.
///     let random_src_segment = Path::new("tests/shaders/random/random_segment.comp");
///
///     // Path to tests for the random segment.
///     let random_test_segments = Path::new("random");
///
///     // Test for `next_u64()`.
///     gen_simple_test_shader!{
///         group_prefix: random_test_segments,
///         shader_name: test_random_next_u64,
///         segments: [random_src_segment]
///     }
///
///     // `test_random_next_u64` is now set to the `Path` to the concatenated file.
/// }
/// ```
///
/// [`vulkano_shaders`]: https://github.com/tomaka/vulkano
/// [`build_glsl_shaders`]: https://github.com/tomaka/vulkano/blob/master/vulkano-shaders/src/lib.rs#L29
/// [this example]: https://github.com/tomaka/vulkano/blob/master/examples/build.rs
///
#[macro_export]
macro_rules! gen_simple_test_shader {
    (
        group_prefix: $group_prefix:ident,
        shader_name: $shader_name:ident,
        segments: [ $( $segment:expr ),* ]
    ) => {
        use std::path::Path;
        use vulkanology::build_utils::concatenate_files;

        let path_and_group = Path::new("tests/shaders").join($group_prefix);
        let segments = [
            path_and_group.join(concat!(stringify!($shader_name), "_header.comp")),
            $( $segment.to_path_buf(), )*
            path_and_group.join(concat!(stringify!($shader_name), "_main.comp"))
        ];
        let output = Path::new("target/test_shaders")
            .join(concat!(stringify!($shader_name), ".comp"));
        concatenate_files(&segments, &output);
        let $shader_name = output.to_str().unwrap();
    }
}
