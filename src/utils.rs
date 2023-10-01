use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
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
pub enum ShaderError {
    FileReadError(String),
    InvalidExtension(String),
    AlreadyIncluded(String),
    WgslParseError(String),
}

impl std::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ShaderError::FileReadError(e) => {
                format!("ShaderError::FileReadError: {}", e)
            }
            ShaderError::InvalidExtension(e) => {
                format!("ShaderError::InvalidExtension: {}", e)
            }
            ShaderError::AlreadyIncluded(e) => {
                format!("ShaderError::AlreadyIncluded: {}", e)
            }
            ShaderError::WgslParseError(e) => {
                format!("ShaderError::WgslParseError: \n{}", e)
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
    pub fn new(source_filepath: &str) -> Result<ShaderModule, ShaderError> {
        let source = ShaderSource::new(source_filepath)?;
        Self::new_from_source(source)
    }

    pub fn new_with_static_sources(
        root_source_path: &str,
        include_srcs: &HashMap<&'static str, &'static str>,
    ) -> Result<ShaderModule, ShaderError> {
        let source = ShaderSource::new_with_static_sources(root_source_path, include_srcs)?;
        Self::new_from_source(source)
    }

    pub fn new_from_source(source: ShaderSource) -> Result<ShaderModule, ShaderError> {
        let mut wgsl_parser = naga::front::wgsl::Frontend::new();
        match wgsl_parser.parse(&source.source) {
            Ok(module) => Ok(ShaderModule {
                module,
            }),
            Err(parse_error) => {
                // ToDo: Fix
                // let mut belonging_parts = vec![];
                // if let Some(location_in_source) = parse_error.location(&source.source) {
                //     for part in source.parts.iter() {
                //         if location_in_source.line_number >= part.start_line as u32
                //             && location_in_source.line_number <= part.end_line as u32
                //         {
                //             belonging_parts.push(part);
                //         }
                //     }
                // }
                // let error_str = if !belonging_parts.is_empty() {
                //     let mut error = "".to_string();
                //     // Take the shallowest matching belonging part
                //     belonging_parts.sort_by(|a, b| a.depth.cmp(&b.depth));
                //     for part in belonging_parts {
                //         error.push_str(
                //             &parse_error.emit_to_string_with_path(&part.content, &part.file_path),
                //         );
                //     }
                //     error
                // } else {
                //     parse_error.emit_to_string_with_path(&source.source, &source.path)
                // };
                let mut error_str = "".to_string();
                error_str
                    .push_str(&parse_error.emit_to_string_with_path(&source.source, &source.path));
                error_str.push_str(&format!(
                    "Included files: {:#?}",
                    source
                        .parts
                        .iter()
                        .map(|p| p.file_path.to_string())
                        .collect::<Vec<String>>()
                ));

                Err(ShaderError::WgslParseError(error_str))
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct ShaderSource {
    pub path: String,
    pub source: String,
    pub parts: Vec<IncludedPart>,
}

impl ShaderSource {
    pub fn new(source_filepath: &str) -> Result<ShaderSource, ShaderError> {
        let mut included_files = HashSet::new();
        let mut file_stack = VecDeque::new();
        let mut included_parts = Vec::new();
        let mut main_file_line_count = 0;

        let path = PathBuf::from(source_filepath);
        let source = wgsl_source_with_includes(
            &path,
            &mut included_files,
            &mut file_stack,
            &mut included_parts,
            &mut main_file_line_count,
            0,
        )?;
        Ok(ShaderSource {
            path: source_filepath.to_string(),
            source,
            parts: included_parts,
        })
    }

    pub fn new_with_static_sources(
        root_source_path: &str,
        include_srcs: &HashMap<&'static str, &'static str>,
    ) -> Result<ShaderSource, ShaderError> {
        let mut included_files = HashSet::new();
        let mut file_stack = VecDeque::new();
        let mut included_parts = Vec::new();
        let mut main_file_line_count = 0;

        let path = PathBuf::from(root_source_path);
        let source = wgsl_source_with_static_includes(
            &path.to_string_lossy().into_owned(),
            include_srcs,
            &mut included_files,
            &mut file_stack,
            &mut included_parts,
            &mut main_file_line_count,
            0,
        )?;
        Ok(ShaderSource {
            path: root_source_path.to_string(),
            source,
            parts: included_parts,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct IncludedPart {
    pub content: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub depth: usize,
}

fn wgsl_source_with_includes(
    file_path: &Path,
    included_files: &mut HashSet<String>,
    file_stack: &mut VecDeque<String>,
    included_parts: &mut Vec<IncludedPart>,
    main_file_line_count: &mut usize,
    depth: usize,
) -> Result<String, ShaderError> {
    let mut result = String::new();
    let file_path_str = file_path.to_string_lossy().into_owned();

    let ext = Path::new(file_path)
        .extension()
        .map(std::ffi::OsStr::to_string_lossy)
        .map(Cow::into_owned);

    if let Some(ext) = ext {
        match ext.as_str() {
            "wgsl" => {}
            e => return Err(ShaderError::InvalidExtension(e.to_string())),
        }
    }

    included_files.insert(file_path_str.clone());
    file_stack.push_back(file_path_str.clone());
    let source = match std::fs::read_to_string(file_path) {
        Ok(str) => str,
        Err(e) => {
            return Err(ShaderError::FileReadError(format!(
                "{}: {}",
                file_path_str, e
            )));
        }
    };

    let current_part_start_line = *main_file_line_count + 1;
    let mut line_count = 0;

    for line in source.lines() {
        if line.starts_with("#include") {
            let included_file_name = line.trim_start_matches("#include ").trim();
            let included_file_path = std::env::current_dir()
                .unwrap()
                .join(included_file_name)
                .clean();

            let included_file_path_str = included_file_path.to_string_lossy().into_owned();
            if included_files.contains(&included_file_path_str) {
                return Err(ShaderError::AlreadyIncluded(format!(
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
                    main_file_line_count,
                    depth + 1,
                )?;

                let part_count = included_part.lines().count();
                included_parts.push(IncludedPart {
                    content: included_part.clone(),
                    file_path: included_file_name.to_string(),
                    start_line: current_part_start_line + line_count,
                    end_line: current_part_start_line + line_count + part_count,
                    depth: depth + 1,
                });

                result.push_str(&included_part);
                line_count += part_count;
            }
        } else {
            result.push_str(line);
            result.push('\n');
            line_count += 1;
        }
    }
    // Remove file from stack after processing
    file_stack.pop_back();

    // Update the line count of the main file
    *main_file_line_count += line_count;

    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn wgsl_source_with_static_includes(
    file_path: &String,
    include_srcs: &HashMap<&'static str, &'static str>,
    included_files: &mut HashSet<String>,
    file_stack: &mut VecDeque<String>,
    included_parts: &mut Vec<IncludedPart>,
    main_file_line_count: &mut usize,
    depth: usize,
) -> Result<String, ShaderError> {
    let mut result = String::new();

    let ext = Path::new(file_path)
        .extension()
        .map(std::ffi::OsStr::to_string_lossy)
        .map(Cow::into_owned);

    if let Some(ext) = ext {
        match ext.as_str() {
            "wgsl" => {}
            e => return Err(ShaderError::InvalidExtension(e.to_string())),
        }
    }

    included_files.insert(file_path.clone());
    file_stack.push_back(file_path.clone());

    let source = match include_srcs.get(file_path.as_str()) {
        Some(str) => str,
        None => {
            return Err(ShaderError::FileReadError(format!(
                "{}: Not found in statically included sources",
                file_path
            )));
        }
    };

    let current_part_start_line = *main_file_line_count + 1;
    let mut line_count = 0;

    for line in source.lines() {
        if line.starts_with("#include") {
            let included_file_name = line.trim_start_matches("#include ").trim().to_string();

            if included_files.contains(&included_file_name) {
                return Err(ShaderError::AlreadyIncluded(format!(
                    "trying to include {} in {}",
                    included_file_name, file_path
                )));
            }
            if !file_stack.contains(&included_file_name) {
                let included_part = wgsl_source_with_static_includes(
                    &included_file_name,
                    include_srcs,
                    included_files,
                    file_stack,
                    included_parts,
                    main_file_line_count,
                    depth + 1,
                )?;
                let part_count = included_part.lines().count();
                included_parts.push(IncludedPart {
                    content: included_part.clone(),
                    file_path: included_file_name,
                    start_line: current_part_start_line + line_count,
                    end_line: current_part_start_line + line_count + part_count,
                    depth: depth + 1,
                });
                result.push_str(&included_part);
                line_count += part_count;
            }
        } else {
            result.push_str(line);
            result.push('\n');
            line_count += 1;
        }
    }
    // Remove file from stack after processing
    file_stack.pop_back();

    // Update the line count of the main file
    *main_file_line_count += line_count;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::utils::{ShaderError, ShaderModule, ShaderSource};

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
        assert!(matches!(result, Err(ShaderError::FileReadError(_))));
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
        assert!(matches!(result, Err(ShaderError::AlreadyIncluded(_))));
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
        assert!(matches!(result, Err(ShaderError::InvalidExtension(_))));
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
        assert!(matches!(result, Err(ShaderError::WgslParseError(_))));
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
        assert!(matches!(result, Err(ShaderError::WgslParseError(_))));
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
        assert!(matches!(result, Err(ShaderError::WgslParseError(_))));
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
        assert!(matches!(result, Err(ShaderError::WgslParseError(_))));
    }
}
