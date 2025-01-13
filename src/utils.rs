use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Formatter,
    future::Future,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use flume::{unbounded, Receiver, Sender};
use log::{error, trace};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use path_clean::PathClean;
use wgpu::naga::Module;

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

pub struct WatchedShaderModule {
    source: ShaderSource,
    _watchers: HashMap<String, Option<RecommendedWatcher>>,
    _receivers: HashMap<String, Receiver<notify::Result<Event>>>,
    first_event_time: Option<Instant>,
    has_pending_changes: bool,
}

impl std::fmt::Debug for WatchedShaderModule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatchedShaderModule")
            .field("source", &self.source)
            .finish()
    }
}

impl WatchedShaderModule {
    pub fn new(path: &Path) -> Result<WatchedShaderModule, ShaderError> {
        let source = ShaderSource::new(path)?;
        Self::new_from_source(source)
    }

    pub fn new_with_static_sources(
        root_source_path: &str,
        include_srcs: &HashMap<&'static str, &'static str>,
    ) -> Result<WatchedShaderModule, ShaderError> {
        let source = ShaderSource::new_with_static_sources(root_source_path, include_srcs)?;
        Self::new_from_source(source)
    }

    pub fn new_from_source(source: ShaderSource) -> Result<WatchedShaderModule, ShaderError> {
        let (watchers, receivers) = if !source.is_static {
            let mut watchers = HashMap::new();
            let mut receivers = HashMap::new();
            for path in Self::paths_to_watch(&source) {
                let (rec, wat) = start_file_watcher(&path);
                watchers.insert(path.clone(), wat);
                receivers.insert(path, rec);
            }
            (watchers, receivers)
        } else {
            trace!("Static shader sources are not watched for {}", source.path);
            (HashMap::default(), HashMap::default())
        };
        Ok(WatchedShaderModule {
            source,
            _watchers: watchers,
            _receivers: receivers,
            first_event_time: None,
            has_pending_changes: false,
        })
    }

    fn paths_to_watch(source: &ShaderSource) -> HashSet<String> {
        let mut paths_to_watch = HashSet::new();
        let path_buf = PathBuf::from(&source.path);
        if path_buf.exists() {
            paths_to_watch.insert(source.path.clone());
        }
        for path in source.parts.iter() {
            let path_buf = PathBuf::from(&path.file_path);
            if path_buf.exists() {
                paths_to_watch.insert(path.file_path.clone());
            }
        }
        paths_to_watch
    }

    pub fn reload(&mut self) -> Result<(), ShaderError> {
        if !self.source.is_static {
            let source = ShaderSource::new(&PathBuf::from(&self.source.path))?;
            let new_watches = Self::paths_to_watch(&source);
            let mut removes = vec![];
            // Find if we need to remove old watchers
            for (key, _) in self._receivers.iter() {
                if !new_watches.contains(key) {
                    removes.push(key.clone());
                }
            }
            // Remove them
            for remove in removes {
                self._receivers.remove(&remove);
                self._watchers.remove(&remove);
            }
            // Insert new watchers
            for path in new_watches {
                if !self._receivers.contains_key(&path) {
                    let (rec, wat) = start_file_watcher(&path);
                    self._receivers.insert(path.clone(), rec);
                    self._watchers.insert(path, wat);
                }
            }
            // Replace source with new
            self.source = source;
        }
        Ok(())
    }

    pub fn reload_with_modified_source(
        &mut self,
        mut modify_fn: impl FnMut(&mut ShaderSource) -> Result<(), ShaderError>,
    ) -> Result<(), ShaderError> {
        self.reload()?;
        modify_fn(&mut self.source)
    }

    pub fn should_reload(&mut self) -> bool {
        for (_path, receiver) in self._receivers.iter() {
            // Process any new events
            for event in receiver.try_iter().flatten() {
                if event
                    .paths
                    .iter()
                    .filter_map(|p| p.to_str())
                    .any(|p| !p.ends_with('~'))
                {
                    self.has_pending_changes = true;
                    if self.first_event_time.is_none() {
                        self.first_event_time = Some(Instant::now());
                    }
                }
            }
        }

        if self.has_pending_changes
            && self
                .first_event_time
                .is_some_and(|t| t.elapsed() >= Duration::from_millis(300))
        {
            self.first_event_time = None;
            self.has_pending_changes = false;
            return true;
        }

        false
    }

    pub fn module(&self) -> Result<ShaderModule, ShaderError> {
        ShaderModule::new_from_source(self.source.clone())
    }

    pub fn paths(&self) -> Vec<&str> {
        let paths = vec![self.source.path.as_str()];
        let other_paths = self
            .source
            .parts
            .iter()
            .map(|p| p.file_path.as_str())
            .collect::<Vec<&str>>();
        [paths, other_paths].concat()
    }
}

fn file_watcher(
    tx: Sender<notify::Result<Event>>,
    path: &str,
) -> notify::Result<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |res| {
        tx.send(res).expect("sending watch event failed");
    })?;
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

pub fn start_file_watcher(
    path: &str,
) -> (Receiver<notify::Result<Event>>, Option<RecommendedWatcher>) {
    let (rx, watcher) = {
        let (tx, rx) = unbounded::<notify::Result<Event>>();
        match file_watcher(tx, path) {
            Ok(watcher) => {
                trace!("Watching {} for changes", path);
                (rx, Some(watcher))
            }
            Err(e) => {
                error!("Shader {} file watcher failed: {:?}", path, e);
                (rx, None)
            }
        }
    };
    (rx, watcher)
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
    pub fn new(path: &Path) -> Result<ShaderModule, ShaderError> {
        let source = ShaderSource::new(path)?;
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
        let mut wgsl_parser = wgpu::naga::front::wgsl::Frontend::new();
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

#[derive(Debug, Default, Clone)]
pub struct ShaderSource {
    pub path: String,
    pub source: String,
    pub parts: Vec<IncludedPart>,
    pub is_static: bool,
}

impl ShaderSource {
    pub fn new(path: &Path) -> Result<ShaderSource, ShaderError> {
        let mut included_files = HashSet::new();
        let mut file_stack = VecDeque::new();
        let mut included_parts = Vec::new();
        let mut main_file_line_count = 0;

        let source = wgsl_source_with_includes(
            path,
            &mut included_files,
            &mut file_stack,
            &mut included_parts,
            &mut main_file_line_count,
            0,
        )?;
        Ok(ShaderSource {
            path: path
                .clean()
                .to_str()
                .unwrap()
                .replace('\\', "/")
                .to_string(),
            source,
            parts: included_parts,
            is_static: false,
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
            path: root_source_path.replace('\\', "/").to_string(),
            source,
            parts: included_parts,
            is_static: true,
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

    let ext = file_path
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
            let included_file_name = line
                .trim_start_matches("#include ")
                .trim()
                .replace('\\', "/");
            let included_file_path = std::env::current_dir()
                .unwrap()
                .join(&included_file_name)
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
    use std::path::PathBuf;

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

        let result = ShaderSource::new(&PathBuf::from(includes_file1));

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

        let result = ShaderSource::new(&PathBuf::from(includes_file1));

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

        let result = ShaderSource::new(&PathBuf::from(includes_file1));

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

        let result = ShaderSource::new(&PathBuf::from(includes_file1));

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

        let result = ShaderModule::new(&PathBuf::from(includes_file1));

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

        let result = ShaderModule::new(&PathBuf::from(includes_file1));

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

        let result = ShaderModule::new(&PathBuf::from(includes_file1));

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

        let result = ShaderModule::new(&PathBuf::from(includes_file1));

        let _ = std::fs::remove_file(includes_file1);
        let _ = std::fs::remove_file(includes_file2);

        assert!(result.is_err());
        assert!(matches!(result, Err(ShaderError::WgslParseError(_))));
    }
}
