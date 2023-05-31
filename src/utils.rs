use std::{
    borrow::Cow,
    collections::{HashSet, VecDeque},
    fmt::Formatter,
    future::Future,
    path::{Path, PathBuf},
};

use naga::Module;
use path_clean::PathClean;

pub fn wait_async<F: Future>(fut: F) -> F::Output {
    pollster::block_on(fut)
}

#[derive(Debug, PartialEq)]
pub enum IncludesShaderError {
    FileReadError(String),
    InvalidExtension(String),
    AlreadyIncluded(String),
    WgslParseError(String),
}

impl std::fmt::Display for IncludesShaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IncludesShaderError::FileReadError(e) => {
                format!("IncludesShaderError::FileReadError: {}", e)
            }
            IncludesShaderError::InvalidExtension(e) => {
                format!("IncludesShaderError::InvalidExtension: {}", e)
            }
            IncludesShaderError::AlreadyIncluded(e) => {
                format!("IncludesShaderError::AlreadyIncluded: {}", e)
            }
            IncludesShaderError::WgslParseError(e) => {
                format!("IncludesShaderError::WgslParseError: \n{}", e)
            }
        };
        write!(f, "{}", s)
    }
}

impl From<ShaderModule> for Module {
    fn from(value: ShaderModule) -> Self {
        value.module
    }
}

#[derive(Debug, Default)]
pub struct ShaderModule {
    module: Module,
}

impl ShaderModule {
    pub fn new(source_filepath: &str) -> Result<ShaderModule, IncludesShaderError> {
        let source = ShaderSource::new(source_filepath)?;
        let mut wgsl_parser = naga::front::wgsl::Frontend::new();
        match wgsl_parser.parse(&source.source) {
            Ok(module) => Ok(ShaderModule {
                module,
            }),
            Err(parse_error) => {
                let mut belonging_part = None;
                if let Some(location_in_source) = parse_error.location(&source.source) {
                    for part in source.parts.iter() {
                        if location_in_source.line_number - 1 == part.end_line as u32
                            || location_in_source.line_number - 1 == part.start_line as u32
                        {
                            belonging_part = Some(part);
                        }
                    }
                }
                let error_str = if let Some(belonging_part) = belonging_part {
                    parse_error.emit_to_string_with_path(
                        &belonging_part.content,
                        &belonging_part.file_path,
                    )
                } else {
                    parse_error.emit_to_string_with_path(&source.source, source_filepath)
                };

                Err(IncludesShaderError::WgslParseError(error_str))
            }
        }
    }
}

#[derive(Debug, Default, Hash, Eq, PartialEq)]
struct SourcePart {
    start: usize,
    end: usize,
}

#[derive(Debug, Default)]
struct ShaderSource {
    source: String,
    parts: Vec<IncludedPart>,
}

impl ShaderSource {
    fn new(source_filepath: &str) -> Result<ShaderSource, IncludesShaderError> {
        let mut included_files = HashSet::new();
        let mut file_stack = VecDeque::new();
        let mut included_parts = Vec::new();

        let path = PathBuf::from(source_filepath);
        let source = wgsl_source_with_includes(
            &path,
            &mut included_files,
            &mut file_stack,
            &mut included_parts,
        )?;
        Ok(ShaderSource {
            source,
            parts: included_parts,
        })
    }
}

#[derive(Debug, Default)]
struct IncludedPart {
    content: String,
    file_path: String,
    start_line: usize,
    end_line: usize,
}

fn wgsl_source_with_includes(
    file_path: &Path,
    included_files: &mut HashSet<String>,
    file_stack: &mut VecDeque<String>,
    included_parts: &mut Vec<IncludedPart>,
) -> Result<String, IncludesShaderError> {
    let mut result = String::new();
    let file_path_str = file_path.to_string_lossy().into_owned();

    let ext = std::path::Path::new(file_path)
        .extension()
        .map(std::ffi::OsStr::to_string_lossy)
        .map(Cow::into_owned);

    if let Some(ext) = ext {
        match ext.as_str() {
            "wgsl" => {}
            e => return Err(IncludesShaderError::InvalidExtension(e.to_string())),
        }
    }

    included_files.insert(file_path_str.clone());
    file_stack.push_back(file_path_str.clone());
    let source = match std::fs::read_to_string(file_path) {
        Ok(str) => str,
        Err(e) => {
            return Err(IncludesShaderError::FileReadError(format!(
                "{}: {}",
                file_path_str, e
            )));
        }
    };

    let mut current_part_start_line = 1;

    for (line_index, line) in source.lines().enumerate() {
        if line.starts_with("#include") {
            let included_file_name = line.trim_start_matches("#include ").trim();
            let included_file_path = file_path.parent().unwrap().join(included_file_name).clean();

            let included_file_path_str = included_file_path.to_string_lossy().into_owned();
            if included_files.contains(&included_file_path_str) {
                return Err(IncludesShaderError::AlreadyIncluded(format!(
                    "trying to include {} in {}",
                    included_file_name, file_path_str
                )));
            }
            if !file_stack.contains(&included_file_path_str) {
                let included_part = wgsl_source_with_includes(
                    &included_file_path,
                    included_files,
                    file_stack,
                    included_parts,
                )?;
                let end_line = current_part_start_line + included_part.lines().count() - 1;
                included_parts.push(IncludedPart {
                    content: included_part.clone(),
                    file_path: included_file_name.to_string(),
                    start_line: current_part_start_line,
                    end_line,
                });
                result.push_str(&included_part);
            }
        } else {
            result.push_str(&line);
            result.push('\n');
        }
        // Track the start line of the next part
        current_part_start_line = line_index + 2;
    }
    // Remove file from stack after processing
    file_stack.pop_back();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::utils::{IncludesShaderError, ShaderModule, ShaderSource};

    #[test]
    fn test_sequentially() {
        test_shader_source();
        test_file_not_found();
        test_file_already_included();
        test_invalid_extension();
        test_shader_parse_error1();
        test_shader_parse_error2();
        test_shader_parse_error3();
        test_shader_parse_error4();
    }

    fn test_shader_source() {
        let includes_file1 = "includes_1.wgsl";
        let includes_file2 = "includes_2.wgsl";
        let includes_file3 = "test_dir/includes_3.wgsl";
        let includes_file4 = "includes_4.wgsl";
        let includes_file5 = "includes_5.wgsl";

        let test_dir = "test_dir";
        let _ = std::fs::create_dir(test_dir);

        let _ = std::fs::write(
            includes_file1,
            r#"
#include includes_2.wgsl"#,
        );
        let _ = std::fs::write(
            includes_file2,
            r#"
#include test_dir/includes_3.wgsl
"#,
        );

        let _ = std::fs::write(
            includes_file3,
            r#"
#include includes_4.wgsl
"#,
        );

        let _ = std::fs::write(
            includes_file4,
            r#"
const TEST: u32 = u32(1);
#include includes_5.wgsl
"#,
        );

        let _ = std::fs::write(
            includes_file5,
            r#"
const TEST2: u32 = u32(2);
"#,
        );

        let result = ShaderSource::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);
        let _ = std::fs::remove_file(includes_file4);
        let _ = std::fs::remove_file(includes_file5);
        let _ = std::fs::remove_dir(test_dir);

        assert!(result.is_ok());
        let result = result.unwrap();
        let should_be = r#"



const TEST: u32 = u32(1);

const TEST2: u32 = u32(2);
"#
        .to_string();
        assert_eq!(result.source, should_be);
    }

    fn test_file_not_found() {
        let includes_file1 = "includes_1.wgsl";
        let includes_file2 = "includes_2.wgsl";

        let _ = std::fs::write(
            includes_file1,
            r#"
#include includes_9.wgsl"#,
        );
        let _ = std::fs::write(
            includes_file2,
            r#"
#include includes_3.wgsl
"#,
        );

        let result = ShaderSource::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);

        assert!(result.is_err());
        assert!(matches!(result, Err(IncludesShaderError::FileReadError(_))));
    }

    fn test_file_already_included() {
        let includes_file1 = "includes_1.wgsl";
        let includes_file2 = "includes_2.wgsl";
        let includes_file3 = "includes_3.wgsl";

        let _ = std::fs::write(
            includes_file1,
            r#"
#include includes_2.wgsl"#,
        );
        let _ = std::fs::write(
            includes_file2,
            r#"
#include includes_1.wgsl
"#,
        );
        let _ = std::fs::write(
            includes_file3,
            r#"
#include includes_2.wgsl
"#,
        );

        let result = ShaderSource::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(IncludesShaderError::AlreadyIncluded(_))
        ));
    }

    fn test_invalid_extension() {
        let includes_file1 = "includes_1.wgsl";

        let _ = std::fs::write(
            includes_file1,
            r#"
#include includes_2.aaa"#,
        );

        let result = ShaderSource::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(IncludesShaderError::InvalidExtension(_))
        ));
    }

    fn test_shader_parse_error1() {
        let includes_file1 = "includes_1.wgsl";
        let includes_file2 = "includes_2.wgsl";
        let includes_file3 = "includes_3.wgsl";

        let _ = std::fs::write(
            includes_file1,
            r#"
#include includes_2.wgsl"#,
        );
        let _ = std::fs::write(
            includes_file2,
            r#"
#include includes_3.wgsl
"#,
        );
        let _ = std::fs::write(
            includes_file3,
            r#"
const TEST1: u32 = u32(1);
const TEST2: u32 = i32(1);
"#,
        );

        let result = ShaderModule::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);

        assert!(result.is_err());
        let should_be = r#"error: the type of `TEST2` is expected to be `u32`, but got `i32`
  ┌─ includes_3.wgsl:3:9
  │
3 │ const TEST2: u32 = i32(1);
  │         ^^^^^ definition of `TEST2`

"#
        .to_string();
        assert_eq!(
            result.unwrap_err(),
            IncludesShaderError::WgslParseError(should_be)
        );
    }

    fn test_shader_parse_error2() {
        let includes_file1 = "includes_1.wgsl";
        let includes_file2 = "includes_2.wgsl";
        let includes_file3 = "includes_3.wgsl";

        let _ = std::fs::write(
            includes_file1,
            r#"
#include includes_2.wgsl"#,
        );
        let _ = std::fs::write(
            includes_file2,
            r#"
const TEST1: u32 = i32(1);
#include includes_3.wgsl
"#,
        );
        let _ = std::fs::write(
            includes_file3,
            r#"
const TEST2: u32 = u32(1);
const TEST3: u32 = i32(1);
"#,
        );

        let result = ShaderModule::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);

        assert!(result.is_err());
        let should_be = r#"error: the type of `TEST1` is expected to be `u32`, but got `i32`
  ┌─ includes_2.wgsl:2:8
  │
2 │ const TEST1: u32 = i32(1);
  │        ^^^^^ definition of `TEST1`

"#
        .to_string();
        assert_eq!(
            result.unwrap_err(),
            IncludesShaderError::WgslParseError(should_be)
        );
    }

    fn test_shader_parse_error3() {
        let includes_file1 = "includes_1.wgsl";
        let includes_file2 = "includes_2.wgsl";
        let includes_file3 = "includes_3.wgsl";

        let _ = std::fs::write(
            includes_file1,
            r#"
const TEST1: u32 = i32(1);
#include includes_2.wgsl
"#,
        );
        let _ = std::fs::write(
            includes_file2,
            r#"
const TEST2: u32 = i32(1);
#include includes_3.wgsl
"#,
        );
        let _ = std::fs::write(
            includes_file3,
            r#"
const TEST3: u32 = u32(1);
const TEST4: u32 = i32(1);
"#,
        );

        let result = ShaderModule::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);

        assert!(result.is_err());
        let should_be = r#"error: the type of `TEST1` is expected to be `u32`, but got `i32`
  ┌─ includes_1.wgsl:2:7
  │
2 │ const TEST1: u32 = i32(1);
  │       ^^^^^ definition of `TEST1`

"#
        .to_string();
        assert_eq!(
            result.unwrap_err(),
            IncludesShaderError::WgslParseError(should_be)
        );
    }

    fn test_shader_parse_error4() {
        let includes_file1 = "includes_1.wgsl";
        let includes_file2 = "includes_2.wgsl";

        let _ = std::fs::write(
            includes_file1,
            r#"
const REDEF: u32 = i32(1);
#include includes_2.wgsl
"#,
        );
        let _ = std::fs::write(
            includes_file2,
            r#"
const REDEF: u32 = i32(1);
"#,
        );

        let result = ShaderModule::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);

        assert!(result.is_err());
        let should_be = r#"error: redefinition of `REDEF`
  ┌─ includes_2.wgsl:2:7
  │
2 │ const REDEF: u32 = i32(1);
  │       ^^^^^ previous definition of `REDEF`
3 │ 
  │   redefinition of `REDEF`

"#
        .to_string();
        assert_eq!(
            result.unwrap_err(),
            IncludesShaderError::WgslParseError(should_be)
        );
    }
}
