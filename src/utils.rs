use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Formatter,
    future::Future,
};

use naga::Module;

pub fn wait_async<F: Future>(fut: F) -> F::Output {
    pollster::block_on(fut)
}

#[derive(Debug)]
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
                format!("IncludesShaderError::WgslParseError: {}", e)
            }
        };
        write!(f, "{}", s)
    }
}

impl From<IncludesShaderModule> for Module {
    fn from(value: IncludesShaderModule) -> Self {
        value.module
    }
}

#[derive(Debug, Default)]
pub struct IncludesShaderModule {
    module: Module,
}

// struc

#[derive(Debug, Default)]
struct ShaderSourceWithIncludeData {
    source: String,
    include_sources: HashMap<String, String>,
    include_data: ShaderFileIncludeData,
}

impl ShaderSourceWithIncludeData {
    fn new(source_filepath: &str) -> Result<ShaderSourceWithIncludeData, IncludesShaderError> {
        let includes_map = wgsl_includes_map(source_filepath, HashMap::new())?;

        let mut include_sources = HashMap::default();
        let source = match std::fs::read_to_string(source_filepath) {
            Ok(str) => str,
            Err(e) => {
                return Err(IncludesShaderError::FileReadError(format!(
                    "{}: {}",
                    source_filepath, e
                )))
            }
        };

        let mut include_data = ShaderFileIncludeData {
            parent_path: source_filepath.to_string(),
            replacements: HashMap::default(),
        };

        for (include_file, (include_path, include_parent_path, replace_line)) in includes_map.iter()
        {
            let parent_key = include_parent_path
                .split("/")
                .last()
                .unwrap_or(&include_parent_path);
            let include_src = match std::fs::read_to_string(include_path.clone()) {
                Ok(str) => str,
                Err(e) => {
                    return Err(IncludesShaderError::FileReadError(format!(
                        "{}: {}",
                        include_path, e
                    )))
                }
            };
            include_sources.insert(parent_key.to_string(), include_src);
            include_data
                .replacements
                .entry(parent_key.to_string())
                .or_insert(HashSet::default())
                .insert(ShaderReplacementData::new(
                    include_parent_path.clone(),
                    include_file.clone(),
                    include_path.clone(),
                    *replace_line,
                ));
        }

        let source_data = ShaderSourceWithIncludeData {
            source,
            include_sources,
            include_data,
        };

        Ok(source_data)
    }
}

#[derive(Debug, Default)]
struct ShaderFileIncludeData {
    pub parent_path: String,
    pub replacements: HashMap<String, HashSet<ShaderReplacementData>>,
}

#[derive(Debug, Default, Hash, Eq, PartialEq)]
struct ShaderReplacementData {
    parent_path: String,
    include_key: String,
    include_path: String,
    replace_line: usize,
}

impl ShaderReplacementData {
    fn new(
        parent_path: String,
        include_key: String,
        include_path: String,
        replace_line: usize,
    ) -> ShaderReplacementData {
        ShaderReplacementData {
            parent_path,
            include_key,
            include_path,
            replace_line,
        }
    }
}

fn wgsl_includes_map(
    source_filepath: &str,
    mut includes_map: HashMap<String, (String, String, usize)>,
) -> Result<HashMap<String, (String, String, usize)>, IncludesShaderError> {
    let source = match std::fs::read_to_string(source_filepath) {
        Ok(str) => str,
        Err(e) => {
            return Err(IncludesShaderError::FileReadError(format!(
                "{}: {}",
                source_filepath, e
            )))
        }
    };

    let ext = std::path::Path::new(source_filepath)
        .extension()
        .map(std::ffi::OsStr::to_string_lossy)
        .map(Cow::into_owned);

    if let Some(ext) = ext {
        match ext.as_str() {
            "wgsl" => {}
            e => return Err(IncludesShaderError::InvalidExtension(e.to_string())),
        }
    }

    for (line_num, line) in source.lines().enumerate() {
        let include_file = line.strip_prefix("#include ");
        if let Some(include_file) = include_file {
            let include_key = include_file.split("/").last().unwrap_or(include_file);
            if includes_map.keys().find(|&k| k == include_key).is_some() {
                return Err(IncludesShaderError::AlreadyIncluded(format!(
                    "file: {} - trying to include: {}",
                    source_filepath, include_file
                )));
            }
            includes_map.insert(
                include_key.to_string(),
                (
                    include_file.to_string(),
                    source_filepath.to_string(),
                    line_num,
                ),
            );
            includes_map = wgsl_includes_map(include_file, includes_map)?;
        }
    }

    Ok(includes_map)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use crate::utils::{
        wgsl_includes_map, IncludesShaderError, ShaderReplacementData, ShaderSourceWithIncludeData,
    };

    #[test]
    fn test_all_sequential() {
        test_includes_set();
        test_circular_reference();
        test_shader_file_with_includes();
        test_shader_file_with_includes_deep();
    }

    fn test_includes_set() {
        let includes_file1 = "includes_1.wgsl";
        let includes_file2 = "includes_2.wgsl";
        let includes_file3 = "includes_3.wgsl";
        let includes_file4 = "includes_4.wgsl";
        let includes_file5 = "includes_5.wgsl";
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
#include includes_4.wgsl
#include includes_5.wgsl
"#,
        );

        let _ = std::fs::write(
            includes_file4,
            r#"
const TEST: u32 = u32(1);
"#,
        );

        let _ = std::fs::write(
            includes_file5,
            r#"
const TEST2: u32 = u32(2);
"#,
        );

        let result = wgsl_includes_map(includes_file1, HashMap::default());

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);
        let _ = std::fs::remove_file(includes_file4);
        let _ = std::fs::remove_file(includes_file5);

        assert!(result.is_ok());
        let mut as_vec = result.unwrap().keys().cloned().collect::<Vec<String>>();
        as_vec.sort();
        let should_be = vec![
            includes_file2,
            includes_file3,
            includes_file4,
            includes_file5,
        ];
        assert_eq!(as_vec, should_be);
    }

    fn test_circular_reference() {
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
#include includes_2.wgsl
"#,
        );

        let _ = std::fs::write(
            includes_file3,
            r#"
const TEST: i32 = 1;
"#,
        );

        let result = wgsl_includes_map(includes_file1, HashMap::default());

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);

        assert!(matches!(
            result,
            Err(IncludesShaderError::AlreadyIncluded(_))
        ));

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
const TEST: i32 = 1;
#include includes_1.wgsl
"#,
        );

        let result = wgsl_includes_map(includes_file1, HashMap::default());

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(IncludesShaderError::AlreadyIncluded(_))
        ));
    }

    fn test_shader_file_with_includes() {
        let includes_file1 = "includes_1.wgsl";
        let includes_file2 = "includes_2.wgsl";
        let includes_file3 = "includes_3.wgsl";
        let includes_file4 = "includes_4.wgsl";
        let includes_file5 = "includes_5.wgsl";
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
#include includes_4.wgsl
#include includes_5.wgsl
"#,
        );

        let _ = std::fs::write(
            includes_file4,
            r#"
const TEST: u32 = u32(1);
"#,
        );

        let _ = std::fs::write(
            includes_file5,
            r#"
const TEST2: u32 = u32(2);
"#,
        );

        let result = ShaderSourceWithIncludeData::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);
        let _ = std::fs::remove_file(includes_file4);
        let _ = std::fs::remove_file(includes_file5);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.include_data.parent_path, includes_file1);
        let mut should_be = HashSet::default();
        should_be.insert(ShaderReplacementData::new(
            includes_file3.to_string(),
            includes_file5.to_string(),
            includes_file5.to_string(),
            2,
        ));
        should_be.insert(ShaderReplacementData::new(
            includes_file3.to_string(),
            includes_file4.to_string(),
            includes_file4.to_string(),
            1,
        ));
        assert_eq!(
            result
                .include_data
                .replacements
                .get(includes_file3)
                .unwrap(),
            &should_be
        );
    }

    fn test_shader_file_with_includes_deep() {
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

        let result = ShaderSourceWithIncludeData::new(includes_file1);

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);
        let _ = std::fs::remove_file(includes_file3);
        let _ = std::fs::remove_file(includes_file4);
        let _ = std::fs::remove_file(includes_file5);
        let _ = std::fs::remove_dir(test_dir);
        println!("{:#?}", result);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.include_data.parent_path, includes_file1);
        let mut should_be = HashSet::default();
        should_be.insert(ShaderReplacementData::new(
            includes_file4.to_string(),
            includes_file5.to_string(),
            includes_file5.to_string(),
            2,
        ));
        assert_eq!(
            result
                .include_data
                .replacements
                .get(includes_file4)
                .unwrap(),
            &should_be
        );

        println!("{:#?}", result);
    }
}
