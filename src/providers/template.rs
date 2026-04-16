//! Template processor for Anvil
//!
//! This module provides template processing using Handlebars, supporting:
//! - Variable expansion (HOME, APPDATA, workload metadata, etc.)
//! - Custom helpers (upper, lower, replace, default, env, date)
//! - Error handling with line number reporting
//! - Preview mode for rendered output

use std::collections::HashMap;
use std::path::Path;

use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext, RenderError,
    RenderErrorReason,
};
use thiserror::Error;

/// Errors that can occur during template processing
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum TemplateError {
    /// Template file not found
    #[error("Template file not found: {0}")]
    NotFound(String),

    /// Template rendering failed
    #[error("Template rendering failed: {message}")]
    RenderFailed {
        message: String,
        #[source]
        source: Option<RenderError>,
    },

    /// Missing required variable
    #[error("Missing required variable: {0}")]
    MissingVariable(String),

    /// Invalid template syntax
    #[error("Invalid template syntax at line {line}: {message}")]
    InvalidSyntax { line: usize, message: String },

    /// IO error while reading/writing template
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for template operations
pub type TemplateResult<T> = Result<T, TemplateError>;

/// Template processor using Handlebars
#[allow(dead_code)]
pub struct TemplateProcessor {
    handlebars: Handlebars<'static>,
    variables: HashMap<String, String>,
}

#[allow(dead_code)]
impl TemplateProcessor {
    /// Create a new template processor with default variables
    pub fn new() -> Self {
        let mut processor = Self {
            handlebars: Handlebars::new(),
            variables: HashMap::new(),
        };

        // Configure handlebars
        processor.handlebars.set_strict_mode(false);
        processor.register_helpers();
        processor.populate_default_variables();

        processor
    }

    /// Create a template processor with a specific workload context
    pub fn with_workload(workload_name: &str, workload_version: &str) -> Self {
        let mut processor = Self::new();
        processor.set_variable("workload_name", workload_name);
        processor.set_variable("workload_version", workload_version);
        processor
    }

    /// Set a template variable
    pub fn set_variable(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }

    /// Set multiple variables from a HashMap
    pub fn set_variables(&mut self, vars: HashMap<String, String>) {
        self.variables.extend(vars);
    }

    /// Get the current variables
    pub fn variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// Populate default system variables
    fn populate_default_variables(&mut self) {
        // User directories
        if let Some(home) = dirs::home_dir() {
            self.set_variable("HOME", &home.to_string_lossy());
            self.set_variable("home", &home.to_string_lossy());
        }

        if let Some(appdata) = dirs::data_dir() {
            self.set_variable("APPDATA", &appdata.to_string_lossy());
            self.set_variable("appdata", &appdata.to_string_lossy());
        }

        if let Some(local_appdata) = dirs::data_local_dir() {
            self.set_variable("LOCALAPPDATA", &local_appdata.to_string_lossy());
            self.set_variable("localappdata", &local_appdata.to_string_lossy());
        }

        if let Some(config) = dirs::config_dir() {
            self.set_variable("CONFIG", &config.to_string_lossy());
            self.set_variable("config", &config.to_string_lossy());
        }

        if let Some(documents) = dirs::document_dir() {
            self.set_variable("DOCUMENTS", &documents.to_string_lossy());
            self.set_variable("documents", &documents.to_string_lossy());
        }

        if let Some(desktop) = dirs::desktop_dir() {
            self.set_variable("DESKTOP", &desktop.to_string_lossy());
            self.set_variable("desktop", &desktop.to_string_lossy());
        }

        // System info
        if let Ok(username) = std::env::var("USERNAME") {
            self.set_variable("USERNAME", &username);
            self.set_variable("username", &username);
        }

        if let Ok(computername) = std::env::var("COMPUTERNAME") {
            self.set_variable("COMPUTERNAME", &computername);
            self.set_variable("computername", &computername);
        }

        if let Ok(temp) = std::env::var("TEMP") {
            self.set_variable("TEMP", &temp);
            self.set_variable("temp", &temp);
        }

        // Program files
        if let Ok(pf) = std::env::var("ProgramFiles") {
            self.set_variable("PROGRAMFILES", &pf);
            self.set_variable("programfiles", &pf);
        }

        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            self.set_variable("PROGRAMFILES_X86", &pf86);
            self.set_variable("programfiles_x86", &pf86);
        }

        // Anvil info
        self.set_variable("anvil_version", env!("CARGO_PKG_VERSION"));
        self.set_variable("ANVIL_VERSION", env!("CARGO_PKG_VERSION"));

        // CPU count
        let cpu_count = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1);
        self.set_variable("cpu_count", &cpu_count.to_string());
        self.set_variable("CPU_COUNT", &cpu_count.to_string());

        // Current date/time
        let now = chrono::Local::now();
        self.set_variable("date", &now.format("%Y-%m-%d").to_string());
        self.set_variable("datetime", &now.format("%Y-%m-%d %H:%M:%S").to_string());
        self.set_variable("timestamp", &now.timestamp().to_string());
    }

    /// Register custom Handlebars helpers
    fn register_helpers(&mut self) {
        // {{upper value}} - Convert to uppercase
        self.handlebars
            .register_helper("upper", Box::new(UpperHelper));

        // {{lower value}} - Convert to lowercase
        self.handlebars
            .register_helper("lower", Box::new(LowerHelper));

        // {{replace value "old" "new"}} - String replacement
        self.handlebars
            .register_helper("replace", Box::new(ReplaceHelper));

        // {{default value "fallback"}} - Default value if empty
        self.handlebars
            .register_helper("default", Box::new(DefaultHelper));

        // {{env "VAR_NAME"}} - Environment variable
        self.handlebars.register_helper("env", Box::new(EnvHelper));

        // {{date "format"}} - Formatted date/time
        self.handlebars
            .register_helper("date", Box::new(DateHelper));

        // {{trim value}} - Trim whitespace
        self.handlebars
            .register_helper("trim", Box::new(TrimHelper));

        // {{concat a b ...}} - Concatenate strings
        self.handlebars
            .register_helper("concat", Box::new(ConcatHelper));

        // {{path_join a b ...}} - Join path components
        self.handlebars
            .register_helper("path_join", Box::new(PathJoinHelper));
    }

    /// Render a template string with the current variables
    pub fn render_string(&self, template: &str) -> TemplateResult<String> {
        self.handlebars
            .render_template(template, &self.variables)
            .map_err(|e| TemplateError::RenderFailed {
                message: e.to_string(),
                source: Some(e),
            })
    }

    /// Render a template file and return the result as a string
    pub fn render_file(&self, path: &Path) -> TemplateResult<String> {
        if !path.exists() {
            return Err(TemplateError::NotFound(path.display().to_string()));
        }

        let template = std::fs::read_to_string(path)?;
        self.render_string(&template)
    }

    /// Render a template file to a destination file
    pub fn render_file_to(&self, source: &Path, destination: &Path) -> TemplateResult<()> {
        let rendered = self.render_file(source)?;

        // Create parent directory if needed
        if let Some(parent) = destination.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        std::fs::write(destination, rendered)?;
        Ok(())
    }

    /// Preview what a template would render to (for dry-run mode)
    pub fn preview(&self, template: &str) -> TemplateResult<TemplatePreview> {
        let rendered = self.render_string(template)?;

        // Find which variables were used
        let used_variables: Vec<String> = self
            .variables
            .keys()
            .filter(|k| template.contains(&format!("{{{{{}}}}}", k)))
            .cloned()
            .collect();

        Ok(TemplatePreview {
            rendered,
            used_variables,
            original_length: template.len(),
        })
    }

    /// Check if a file should be treated as a template
    pub fn is_template_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            return ext == "hbs" || ext == "handlebars" || ext == "template";
        }
        false
    }

    /// Get the output filename for a template file (strips .hbs extension)
    pub fn output_filename(path: &Path) -> std::path::PathBuf {
        if Self::is_template_file(path) {
            if let Some(stem) = path.file_stem() {
                if let Some(parent) = path.parent() {
                    return parent.join(stem);
                }
                return std::path::PathBuf::from(stem);
            }
        }
        path.to_path_buf()
    }

    /// Validate a template for syntax errors without rendering
    pub fn validate(&self, template: &str) -> TemplateResult<()> {
        // Try to compile the template
        self.handlebars
            .render_template(template, &serde_json::json!({}))
            .map_err(|e| {
                // Try to extract line number from error
                let message = e.to_string();
                TemplateError::RenderFailed {
                    message,
                    source: Some(e),
                }
            })?;
        Ok(())
    }
}

impl Default for TemplateProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Preview information for a rendered template
#[allow(dead_code)]
#[derive(Debug)]
pub struct TemplatePreview {
    /// The rendered output
    pub rendered: String,
    /// Variables that were used in the template
    pub used_variables: Vec<String>,
    /// Original template length
    pub original_length: usize,
}

#[allow(dead_code)]
impl TemplatePreview {
    /// Get the rendered length
    pub fn rendered_length(&self) -> usize {
        self.rendered.len()
    }
}

// ============================================================================
// Custom Handlebars Helpers
// ============================================================================

/// Helper: {{upper value}} - Convert string to uppercase
struct UpperHelper;

impl HelperDef for UpperHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
        out.write(&param.to_uppercase())?;
        Ok(())
    }
}

/// Helper: {{lower value}} - Convert string to lowercase
struct LowerHelper;

impl HelperDef for LowerHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
        out.write(&param.to_lowercase())?;
        Ok(())
    }
}

/// Helper: {{replace value "old" "new"}} - Replace occurrences of a string
struct ReplaceHelper;

impl HelperDef for ReplaceHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let value = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
        let old = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
        let new = h.param(2).and_then(|v| v.value().as_str()).unwrap_or("");

        out.write(&value.replace(old, new))?;
        Ok(())
    }
}

/// Helper: {{default value "fallback"}} - Use fallback if value is empty
struct DefaultHelper;

impl HelperDef for DefaultHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let value = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
        let fallback = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");

        if value.is_empty() {
            out.write(fallback)?;
        } else {
            out.write(value)?;
        }
        Ok(())
    }
}

/// Helper: {{env "VAR_NAME"}} - Get environment variable
struct EnvHelper;

impl HelperDef for EnvHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let var_name = h
            .param(0)
            .and_then(|v| v.value().as_str())
            .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("env", 0))?;

        let value = std::env::var(var_name).unwrap_or_default();
        out.write(&value)?;
        Ok(())
    }
}

/// Helper: {{date "format"}} - Format current date/time
struct DateHelper;

impl HelperDef for DateHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let format = h
            .param(0)
            .and_then(|v| v.value().as_str())
            .unwrap_or("%Y-%m-%d");

        let now = chrono::Local::now();
        out.write(&now.format(format).to_string())?;
        Ok(())
    }
}

/// Helper: {{trim value}} - Trim whitespace from string
struct TrimHelper;

impl HelperDef for TrimHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let value = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
        out.write(value.trim())?;
        Ok(())
    }
}

/// Helper: {{concat a b c ...}} - Concatenate multiple strings
struct ConcatHelper;

impl HelperDef for ConcatHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let mut result = String::new();
        for param in h.params() {
            if let Some(s) = param.value().as_str() {
                result.push_str(s);
            }
        }
        out.write(&result)?;
        Ok(())
    }
}

/// Helper: {{path_join a b c ...}} - Join path components
struct PathJoinHelper;

impl HelperDef for PathJoinHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let mut path = std::path::PathBuf::new();
        for param in h.params() {
            if let Some(s) = param.value().as_str() {
                path.push(s);
            }
        }
        out.write(&path.to_string_lossy())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_template() {
        let processor = TemplateProcessor::new();
        let result = processor.render_string("Hello, {{username}}!").unwrap();
        // Should contain some username or empty string
        assert!(result.starts_with("Hello, "));
    }

    #[test]
    fn test_upper_helper() {
        let mut processor = TemplateProcessor::new();
        processor.set_variable("name", "world");
        let result = processor.render_string("{{upper name}}").unwrap();
        assert_eq!(result, "WORLD");
    }

    #[test]
    fn test_lower_helper() {
        let mut processor = TemplateProcessor::new();
        processor.set_variable("name", "WORLD");
        let result = processor.render_string("{{lower name}}").unwrap();
        assert_eq!(result, "world");
    }

    #[test]
    fn test_replace_helper() {
        let mut processor = TemplateProcessor::new();
        processor.set_variable("text", "hello world");
        let result = processor
            .render_string("{{replace text \"world\" \"rust\"}}")
            .unwrap();
        assert_eq!(result, "hello rust");
    }

    #[test]
    fn test_default_helper() {
        let processor = TemplateProcessor::new();
        // Empty variable should use default
        let result = processor
            .render_string("{{default missing \"fallback\"}}")
            .unwrap();
        assert_eq!(result, "fallback");
    }

    #[test]
    fn test_date_helper() {
        let processor = TemplateProcessor::new();
        let result = processor.render_string("{{date \"%Y\"}}").unwrap();
        let year = chrono::Local::now().format("%Y").to_string();
        assert_eq!(result, year);
    }

    #[test]
    fn test_concat_helper() {
        let mut processor = TemplateProcessor::new();
        processor.set_variable("a", "Hello");
        processor.set_variable("b", " ");
        processor.set_variable("c", "World");
        let result = processor.render_string("{{concat a b c}}").unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_is_template_file() {
        assert!(TemplateProcessor::is_template_file(Path::new("config.hbs")));
        assert!(TemplateProcessor::is_template_file(Path::new(
            "config.handlebars"
        )));
        assert!(TemplateProcessor::is_template_file(Path::new(
            "config.template"
        )));
        assert!(!TemplateProcessor::is_template_file(Path::new(
            "config.txt"
        )));
        assert!(!TemplateProcessor::is_template_file(Path::new(
            "config.yaml"
        )));
    }

    #[test]
    fn test_output_filename() {
        assert_eq!(
            TemplateProcessor::output_filename(Path::new("config.toml.hbs")),
            Path::new("config.toml")
        );
        assert_eq!(
            TemplateProcessor::output_filename(Path::new("dir/config.yaml.hbs")),
            Path::new("dir/config.yaml")
        );
        assert_eq!(
            TemplateProcessor::output_filename(Path::new("config.txt")),
            Path::new("config.txt")
        );
    }

    #[test]
    fn test_workload_context() {
        let processor = TemplateProcessor::with_workload("rust-dev", "1.0.0");
        let result = processor
            .render_string("Workload: {{workload_name}} v{{workload_version}}")
            .unwrap();
        assert_eq!(result, "Workload: rust-dev v1.0.0");
    }

    #[test]
    fn test_preview() {
        let mut processor = TemplateProcessor::new();
        processor.set_variable("name", "test");
        let preview = processor.preview("Hello {{name}}!").unwrap();
        assert_eq!(preview.rendered, "Hello test!");
    }

    #[test]
    fn test_trim_helper() {
        let mut processor = TemplateProcessor::new();
        processor.set_variable("text", "  hello  ");
        let result = processor.render_string("{{trim text}}").unwrap();
        assert_eq!(result, "hello");
    }
}
