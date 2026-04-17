//! Workload inheritance resolution module
//!
//! This module handles resolving workload inheritance chains,
//! merging parent workloads with child workloads according to
//! defined merge strategies.

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, trace};

use super::workload::{
    AptPackage, BrewPackage, CommandBlock, CommandEntry, Environment, FileEntry, HealthCheckScript,
    Packages, ScriptEntry, Scripts, WingetPackage, Workload,
};
use super::ConfigManager;

/// Maximum depth for inheritance chains to prevent runaway recursion
const MAX_INHERITANCE_DEPTH: usize = 10;

/// Graph representation of workload inheritance relationships
///
/// This structure provides visualization and analysis of inheritance chains,
/// including cycle detection and tree formatting.
#[derive(Debug, Default)]
pub struct InheritanceGraph {
    /// Maps workload name to its parent names
    edges: HashMap<String, Vec<String>>,
    /// Topologically sorted order for resolution (leaf to root)
    resolution_order: Vec<String>,
    /// All workloads in the graph
    workloads: HashSet<String>,
}

impl InheritanceGraph {
    /// Create a new empty inheritance graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an inheritance graph starting from a root workload
    pub fn build(root: &str, manager: &mut ConfigManager) -> Result<Self, InheritanceError> {
        let mut graph = Self::new();
        let mut visited = HashSet::new();
        let mut path_stack = Vec::new();

        graph.traverse(root, manager, &mut visited, &mut path_stack, 0)?;
        graph.topological_sort()?;

        Ok(graph)
    }

    /// Recursively traverse and build the graph
    fn traverse(
        &mut self,
        workload_name: &str,
        manager: &mut ConfigManager,
        visited: &mut HashSet<String>,
        path_stack: &mut Vec<String>,
        depth: usize,
    ) -> Result<(), InheritanceError> {
        // Check max depth
        if depth > MAX_INHERITANCE_DEPTH {
            return Err(InheritanceError::MaxDepthExceeded {
                depth: MAX_INHERITANCE_DEPTH,
                workload: workload_name.to_string(),
            });
        }

        // Check for cycles
        if path_stack.contains(&workload_name.to_string()) {
            let mut cycle = path_stack.clone();
            cycle.push(workload_name.to_string());
            return Err(InheritanceError::CircularDependency {
                chain: cycle.join(" -> "),
            });
        }

        // Already fully processed
        if visited.contains(workload_name) {
            return Ok(());
        }

        // Add to path stack for cycle detection
        path_stack.push(workload_name.to_string());
        self.workloads.insert(workload_name.to_string());

        // Find and load workload
        let workload_path = manager.find_workload(workload_name).ok_or_else(|| {
            InheritanceError::ParentNotFound {
                parent: workload_name.to_string(),
            }
        })?;

        let workload =
            load_workload_from_path(&workload_path).map_err(|e| InheritanceError::LoadError {
                workload: workload_name.to_string(),
                reason: e.to_string(),
            })?;

        // Get parents
        let parents = workload.extends.as_ref().cloned().unwrap_or_default();

        // Store edges
        self.edges
            .insert(workload_name.to_string(), parents.clone());

        // Recursively process parents
        for parent in parents {
            self.traverse(&parent, manager, visited, path_stack, depth + 1)?;
        }

        // Mark as visited and remove from path stack
        visited.insert(workload_name.to_string());
        path_stack.pop();

        Ok(())
    }

    /// Perform topological sort to determine resolution order
    fn topological_sort(&mut self) -> Result<(), InheritanceError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut reverse_edges: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize in-degrees
        for workload in &self.workloads {
            in_degree.insert(workload.clone(), 0);
        }

        // Calculate in-degrees and build reverse edges
        for (child, parents) in &self.edges {
            for parent in parents {
                *in_degree.entry(child.clone()).or_insert(0) += 1;
                reverse_edges
                    .entry(parent.clone())
                    .or_default()
                    .push(child.clone());
            }
        }

        // Kahn's algorithm
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop() {
            result.push(node.clone());

            if let Some(children) = reverse_edges.get(&node) {
                for child in children {
                    if let Some(deg) = in_degree.get_mut(child) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(child.clone());
                        }
                    }
                }
            }
        }

        // Reverse to get leaf-to-root order (parents first)
        result.reverse();
        self.resolution_order = result;

        Ok(())
    }

    /// Format the inheritance graph as an ASCII tree
    pub fn format_tree(&self, root: &str) -> String {
        let mut output = String::new();
        self.format_tree_recursive(root, &mut output, "", true);
        output
    }

    /// Recursive helper for tree formatting
    fn format_tree_recursive(&self, node: &str, output: &mut String, prefix: &str, is_last: bool) {
        // Add current node
        let connector = if prefix.is_empty() {
            ""
        } else if is_last {
            "└── "
        } else {
            "├── "
        };
        output.push_str(&format!("{}{}{}\n", prefix, connector, node));

        // Get children (workloads that extend this one)
        let parents = self.edges.get(node).cloned().unwrap_or_default();

        if parents.is_empty() {
            return;
        }

        // Prepare prefix for children
        let child_prefix = if prefix.is_empty() {
            "".to_string()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        // Process parents (shown as children in the tree)
        for (i, parent) in parents.iter().enumerate() {
            let is_last_child = i == parents.len() - 1;
            self.format_tree_recursive(parent, output, &child_prefix, is_last_child);
        }
    }

    /// Get the resolution order (parents before children)
    #[allow(dead_code)]
    pub fn resolution_order(&self) -> &[String] {
        &self.resolution_order
    }

    /// Get all workloads in the graph
    #[allow(dead_code)]
    pub fn workloads(&self) -> &HashSet<String> {
        &self.workloads
    }

    /// Get the parents of a workload
    #[allow(dead_code)]
    pub fn parents(&self, workload: &str) -> Option<&Vec<String>> {
        self.edges.get(workload)
    }

    /// Get the depth of a workload in the inheritance chain
    pub fn depth(&self, workload: &str) -> usize {
        let parents = self.edges.get(workload);
        match parents {
            None => 0,
            Some(parents) if parents.is_empty() => 0,
            Some(parents) => 1 + parents.iter().map(|p| self.depth(p)).max().unwrap_or(0),
        }
    }

    /// Check if the graph has any cycles (should always be false after successful build)
    #[allow(dead_code)]
    pub fn has_cycle(&self) -> bool {
        self.resolution_order.len() != self.workloads.len()
    }

    /// Get statistics about the inheritance graph
    pub fn stats(&self) -> InheritanceStats {
        let total_workloads = self.workloads.len();
        let max_depth = self
            .workloads
            .iter()
            .map(|w| self.depth(w))
            .max()
            .unwrap_or(0);
        let total_edges: usize = self.edges.values().map(|v| v.len()).sum();

        InheritanceStats {
            total_workloads,
            max_depth,
            total_edges,
        }
    }
}

/// Statistics about an inheritance graph
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InheritanceStats {
    /// Total number of workloads
    pub total_workloads: usize,
    /// Maximum inheritance depth
    pub max_depth: usize,
    /// Total number of inheritance edges
    pub total_edges: usize,
}

/// Error types specific to inheritance resolution
#[derive(Debug, thiserror::Error)]
pub enum InheritanceError {
    #[error("Circular dependency detected: {chain}")]
    CircularDependency { chain: String },

    #[error("Maximum inheritance depth ({depth}) exceeded for workload '{workload}'")]
    MaxDepthExceeded { depth: usize, workload: String },

    #[error("Parent workload '{parent}' not found")]
    ParentNotFound { parent: String },

    #[error("Failed to load workload '{workload}': {reason}")]
    LoadError { workload: String, reason: String },
}

impl InheritanceError {
    /// Get a user-friendly suggestion for fixing the error
    pub fn suggestion(&self) -> String {
        match self {
            InheritanceError::CircularDependency { chain } => {
                format!(
                    "Remove one of the 'extends' references to break the cycle.\n\
                     Detected chain: {}",
                    chain
                )
            }
            InheritanceError::MaxDepthExceeded { depth, .. } => {
                format!(
                    "Consider flattening your inheritance hierarchy.\n\
                     Maximum allowed depth is {}.",
                    depth
                )
            }
            InheritanceError::ParentNotFound { parent } => {
                format!(
                    "Ensure the workload '{}' exists in one of the search paths.\n\
                     Run 'anvil list' to see available workloads.",
                    parent
                )
            }
            InheritanceError::LoadError { workload, reason } => {
                format!(
                    "Check the workload file for '{}' for syntax errors.\n\
                     Error: {}",
                    workload, reason
                )
            }
        }
    }
}

/// Resolve inheritance for a workload
///
/// This function recursively loads and merges parent workloads
/// to produce a fully resolved workload definition.
///
/// # Merge Strategies
///
/// | Field | Strategy |
/// |-------|----------|
/// | name | Child overwrites |
/// | version | Child overwrites |
/// | description | Child overwrites |
/// | packages.winget | Append (child after parent) |
/// | files | Append (same destinations overwritten by child) |
/// | scripts.pre_install | Parent first, then child |
/// | scripts.post_install | Parent first, then child |
/// | scripts.health_check | Combine all |
/// | environment.variables | Child overwrites same-named |
/// | environment.path_additions | Append |
///
/// # Examples
///
/// ```ignore
/// let mut manager = ConfigManager::new();
/// let workload = manager.load_workload(&path)?;
/// let resolved = resolve_inheritance(&mut manager, workload)?;
/// ```
pub fn resolve_inheritance(manager: &mut ConfigManager, workload: Workload) -> Result<Workload> {
    let mut visited = HashSet::new();
    let mut path_stack = Vec::new();

    debug!(
        "Starting inheritance resolution for workload: {}",
        workload.name
    );

    resolve_inheritance_recursive(manager, workload, &mut visited, &mut path_stack, 0)
}

/// Recursive implementation of inheritance resolution
fn resolve_inheritance_recursive(
    manager: &mut ConfigManager,
    workload: Workload,
    visited: &mut HashSet<String>,
    path_stack: &mut Vec<String>,
    depth: usize,
) -> Result<Workload> {
    trace!(
        "resolve_inheritance_recursive: workload={}, depth={}",
        workload.name,
        depth
    );

    // Check for maximum depth
    if depth > MAX_INHERITANCE_DEPTH {
        return Err(InheritanceError::MaxDepthExceeded {
            depth: MAX_INHERITANCE_DEPTH,
            workload: workload.name.clone(),
        }
        .into());
    }

    // Check for circular dependencies
    if visited.contains(&workload.name) {
        path_stack.push(workload.name.clone());
        let chain = path_stack.join(" -> ");
        return Err(InheritanceError::CircularDependency { chain }.into());
    }

    // Mark this workload as visited and add to path stack
    visited.insert(workload.name.clone());
    path_stack.push(workload.name.clone());

    // If no parents, return the workload as-is
    let parents = match &workload.extends {
        Some(extends) if !extends.is_empty() => extends.clone(),
        _ => {
            debug!(
                "Workload '{}' has no parents, returning as-is",
                workload.name
            );
            // Clean up before returning
            visited.remove(&workload.name);
            path_stack.pop();
            return Ok(workload);
        }
    };

    debug!("Workload '{}' extends: {:?}", workload.name, parents);

    // Start with an empty base and merge parents in order
    let mut merged = Workload::empty();

    for parent_name in &parents {
        debug!("Loading parent workload: {}", parent_name);

        // Find the parent workload path
        let parent_path =
            manager
                .find_workload(parent_name)
                .ok_or_else(|| InheritanceError::ParentNotFound {
                    parent: parent_name.clone(),
                })?;

        // Load the parent workload (without resolving - we'll do that recursively)
        let parent = load_workload_from_path(&parent_path)
            .with_context(|| format!("Failed to load parent workload: {}", parent_name))?;

        // Recursively resolve the parent's inheritance
        let resolved_parent =
            resolve_inheritance_recursive(manager, parent, visited, path_stack, depth + 1)?;

        // Merge the resolved parent into our merged workload
        merged = merge_workloads(merged, resolved_parent);

        trace!("Merged parent '{}' into accumulated result", parent_name);
    }

    // Finally, merge the child workload on top
    let result = merge_workloads(merged, workload.clone());

    debug!(
        "Inheritance resolution complete for '{}': {} packages, {} files",
        result.name,
        result.package_count(),
        result.file_count()
    );

    // Remove from visited set and path stack when we're done
    visited.remove(&workload.name);
    path_stack.pop();

    Ok(result)
}

/// Load a workload from a file path without going through ConfigManager
/// This avoids issues with recursive load_resolved calls
fn load_workload_from_path(path: &PathBuf) -> Result<Workload> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read workload file: {}", path.display()))?;

    let workload: Workload = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse workload file: {}", path.display()))?;

    Ok(workload)
}

/// Merge two workloads according to the merge strategies
///
/// The `child` workload takes precedence over the `parent` workload.
fn merge_workloads(parent: Workload, child: Workload) -> Workload {
    Workload {
        // Child overwrites basic fields
        name: if child.name.is_empty() {
            parent.name
        } else {
            child.name
        },
        version: if child.version.is_empty() {
            parent.version
        } else {
            child.version
        },
        description: if child.description.is_empty() {
            parent.description
        } else {
            child.description
        },

        // Don't inherit extends field - the resolved workload doesn't need it
        extends: None,

        // Merge packages
        packages: merge_packages(parent.packages, child.packages),

        // Merge files (child can override same destinations)
        files: merge_files(parent.files, child.files),

        // Merge scripts
        scripts: merge_scripts(parent.scripts, child.scripts),

        // Merge commands
        commands: merge_commands(parent.commands, child.commands),

        // Merge environment
        environment: merge_environment(parent.environment, child.environment),

        // Child overwrites health config, or use parent if child has none
        health: child.health.or(parent.health),

        // Merge assertions: append child after parent
        assertions: match (parent.assertions, child.assertions) {
            (None, None) => None,
            (Some(p), None) => Some(p),
            (None, Some(c)) => Some(c),
            (Some(mut p), Some(c)) => {
                p.extend(c);
                Some(p)
            }
        },
    }
}

/// Merge package definitions
///
/// Strategy: Append child packages after parent packages.
/// Duplicate package IDs are preserved (child's version will be installed).
fn merge_packages(parent: Option<Packages>, child: Option<Packages>) -> Option<Packages> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (Some(parent), Some(child)) => {
            let mut result = Packages {
                winget: None,
                brew: None,
                apt: None,
            };

            // Merge winget packages
            let parent_winget = parent.winget.unwrap_or_default();
            let child_winget = child.winget.unwrap_or_default();

            if !parent_winget.is_empty() || !child_winget.is_empty() {
                let mut merged: Vec<WingetPackage> = parent_winget;

                // Add child packages, but don't duplicate IDs
                for child_pkg in child_winget {
                    // Check if this package ID already exists
                    if let Some(existing) = merged.iter_mut().find(|p| p.id == child_pkg.id) {
                        // Replace with child's definition (child takes precedence)
                        *existing = child_pkg;
                    } else {
                        // Add new package
                        merged.push(child_pkg);
                    }
                }

                result.winget = Some(merged);
            }

            // Merge brew packages
            let parent_brew = parent.brew.unwrap_or_default();
            let child_brew = child.brew.unwrap_or_default();

            if !parent_brew.is_empty() || !child_brew.is_empty() {
                let mut merged: Vec<BrewPackage> = parent_brew;

                for child_pkg in child_brew {
                    if let Some(existing) = merged.iter_mut().find(|p| p.name == child_pkg.name) {
                        *existing = child_pkg;
                    } else {
                        merged.push(child_pkg);
                    }
                }

                result.brew = Some(merged);
            }

            // Merge apt packages
            let parent_apt = parent.apt.unwrap_or_default();
            let child_apt = child.apt.unwrap_or_default();

            if !parent_apt.is_empty() || !child_apt.is_empty() {
                let mut merged: Vec<AptPackage> = parent_apt;

                for child_pkg in child_apt {
                    if let Some(existing) = merged.iter_mut().find(|p| p.name == child_pkg.name) {
                        *existing = child_pkg;
                    } else {
                        merged.push(child_pkg);
                    }
                }

                result.apt = Some(merged);
            }

            Some(result)
        }
    }
}

/// Merge file definitions
///
/// Strategy: Append child files after parent files.
/// Files with the same destination are overwritten by the child.
fn merge_files(
    parent: Option<Vec<FileEntry>>,
    child: Option<Vec<FileEntry>>,
) -> Option<Vec<FileEntry>> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (Some(mut parent), Some(child)) => {
            // For files with the same destination, child overwrites
            for child_file in child {
                if let Some(pos) = parent
                    .iter()
                    .position(|f| f.destination == child_file.destination)
                {
                    // Replace parent file with child file
                    parent[pos] = child_file;
                } else {
                    // Add new file
                    parent.push(child_file);
                }
            }
            Some(parent)
        }
    }
}

/// Merge script definitions
///
/// Strategy:
/// - pre_install: Parent scripts first, then child scripts
/// - post_install: Parent scripts first, then child scripts
/// - health_check: Combine all (parent first, then child)
fn merge_scripts(parent: Option<Scripts>, child: Option<Scripts>) -> Option<Scripts> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (Some(parent), Some(child)) => {
            let mut result = Scripts {
                pre_install: None,
                post_install: None,
                health_check: None,
            };

            // Pre-install: parent first, then child
            result.pre_install = merge_script_list(parent.pre_install, child.pre_install);

            // Post-install: parent first, then child
            result.post_install = merge_script_list(parent.post_install, child.post_install);

            // Health check: combine all (using merge_health_check_list)
            result.health_check = merge_health_check_list(parent.health_check, child.health_check);

            // Only return Some if at least one script list is present
            if result.pre_install.is_some()
                || result.post_install.is_some()
                || result.health_check.is_some()
            {
                Some(result)
            } else {
                None
            }
        }
    }
}

/// Merge two script entry lists (parent first, then child)
fn merge_script_list(
    parent: Option<Vec<ScriptEntry>>,
    child: Option<Vec<ScriptEntry>>,
) -> Option<Vec<ScriptEntry>> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (Some(mut parent), Some(child)) => {
            parent.extend(child);
            Some(parent)
        }
    }
}

/// Merge two health check script lists
fn merge_health_check_list(
    parent: Option<Vec<HealthCheckScript>>,
    child: Option<Vec<HealthCheckScript>>,
) -> Option<Vec<HealthCheckScript>> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (Some(mut parent), Some(child)) => {
            parent.extend(child);
            Some(parent)
        }
    }
}

/// Merge command block definitions
///
/// Strategy:
/// - pre_install: Parent commands first, then child commands appended
/// - post_install: Parent commands first, then child commands appended
fn merge_commands(
    parent: Option<CommandBlock>,
    child: Option<CommandBlock>,
) -> Option<CommandBlock> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (Some(parent), Some(child)) => {
            let mut result = CommandBlock {
                pre_install: None,
                post_install: None,
            };

            // Pre-install: parent first, then child
            result.pre_install = merge_command_list(parent.pre_install, child.pre_install);

            // Post-install: parent first, then child
            result.post_install = merge_command_list(parent.post_install, child.post_install);

            // Only return Some if at least one command list is present
            if result.pre_install.is_some() || result.post_install.is_some() {
                Some(result)
            } else {
                None
            }
        }
    }
}

/// Merge two command entry lists (parent first, then child)
fn merge_command_list(
    parent: Option<Vec<CommandEntry>>,
    child: Option<Vec<CommandEntry>>,
) -> Option<Vec<CommandEntry>> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (Some(mut parent), Some(child)) => {
            parent.extend(child);
            Some(parent)
        }
    }
}

/// Merge environment definitions
///
/// Strategy:
/// - variables: Child overwrites same-named variables
/// - path_additions: Append (parent first, then child)
fn merge_environment(
    parent: Option<Environment>,
    child: Option<Environment>,
) -> Option<Environment> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (Some(parent), Some(child)) => {
            let mut result = Environment {
                variables: None,
                path_additions: None,
            };

            // Variables: child overwrites same-named
            match (parent.variables, child.variables) {
                (None, None) => {}
                (Some(p), None) => result.variables = Some(p),
                (None, Some(c)) => result.variables = Some(c),
                (Some(mut parent_vars), Some(child_vars)) => {
                    for child_var in child_vars {
                        if let Some(pos) = parent_vars.iter().position(|v| v.name == child_var.name)
                        {
                            // Replace with child's variable
                            parent_vars[pos] = child_var;
                        } else {
                            // Add new variable
                            parent_vars.push(child_var);
                        }
                    }
                    result.variables = Some(parent_vars);
                }
            }

            // Path additions: append (parent first, then child)
            match (parent.path_additions, child.path_additions) {
                (None, None) => {}
                (Some(p), None) => result.path_additions = Some(p),
                (None, Some(c)) => result.path_additions = Some(c),
                (Some(mut parent_paths), Some(child_paths)) => {
                    // Avoid duplicates while preserving order
                    for path in child_paths {
                        if !parent_paths.contains(&path) {
                            parent_paths.push(path);
                        }
                    }
                    result.path_additions = Some(parent_paths);
                }
            }

            // Only return Some if at least one field is present
            if result.variables.is_some() || result.path_additions.is_some() {
                Some(result)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::workload::{EnvVariable, FileEntry, WingetPackage};

    #[test]
    fn test_inheritance_graph_format_tree() {
        let mut graph = InheritanceGraph::new();
        graph.workloads.insert("rust-developer".to_string());
        graph.workloads.insert("essentials".to_string());
        graph.workloads.insert("base-workstation".to_string());

        graph
            .edges
            .insert("rust-developer".to_string(), vec!["essentials".to_string()]);
        graph.edges.insert(
            "essentials".to_string(),
            vec!["base-workstation".to_string()],
        );
        graph.edges.insert("base-workstation".to_string(), vec![]);

        let tree = graph.format_tree("rust-developer");
        assert!(tree.contains("rust-developer"));
        assert!(tree.contains("essentials"));
        assert!(tree.contains("base-workstation"));
    }

    #[test]
    fn test_inheritance_graph_depth() {
        let mut graph = InheritanceGraph::new();
        graph.workloads.insert("child".to_string());
        graph.workloads.insert("parent".to_string());
        graph.workloads.insert("grandparent".to_string());

        graph
            .edges
            .insert("child".to_string(), vec!["parent".to_string()]);
        graph
            .edges
            .insert("parent".to_string(), vec!["grandparent".to_string()]);
        graph.edges.insert("grandparent".to_string(), vec![]);

        assert_eq!(graph.depth("grandparent"), 0);
        assert_eq!(graph.depth("parent"), 1);
        assert_eq!(graph.depth("child"), 2);
    }

    #[test]
    fn test_inheritance_error_suggestions() {
        let err = InheritanceError::CircularDependency {
            chain: "A -> B -> A".to_string(),
        };
        assert!(err.suggestion().contains("Remove one of the 'extends'"));

        let err = InheritanceError::ParentNotFound {
            parent: "missing".to_string(),
        };
        assert!(err.suggestion().contains("missing"));
    }

    #[test]
    fn test_merge_empty_workloads() {
        let parent = Workload::empty();
        let child = Workload {
            name: "child".to_string(),
            version: "1.0.0".to_string(),
            description: "Child workload".to_string(),
            ..Workload::empty()
        };

        let merged = merge_workloads(parent, child);
        assert_eq!(merged.name, "child");
        assert_eq!(merged.version, "1.0.0");
        assert_eq!(merged.description, "Child workload");
    }

    #[test]
    fn test_merge_preserves_parent_when_child_empty() {
        let parent = Workload {
            name: "parent".to_string(),
            version: "1.0.0".to_string(),
            description: "Parent workload".to_string(),
            ..Workload::empty()
        };
        let child = Workload::empty();

        let merged = merge_workloads(parent, child);
        assert_eq!(merged.name, "parent");
        assert_eq!(merged.version, "1.0.0");
        assert_eq!(merged.description, "Parent workload");
    }

    #[test]
    fn test_merge_packages() {
        let parent = Some(Packages {
            winget: Some(vec![
                WingetPackage::new("Parent.Package1"),
                WingetPackage::new("Parent.Package2"),
            ]),
            ..Default::default()
        });
        let child = Some(Packages {
            winget: Some(vec![
                WingetPackage::new("Child.Package1"),
                WingetPackage::with_version("Parent.Package2", "2.0.0"), // Override
            ]),
            ..Default::default()
        });

        let merged = merge_packages(parent, child).unwrap();
        let winget = merged.winget.unwrap();

        assert_eq!(winget.len(), 3);
        assert_eq!(winget[0].id, "Parent.Package1");
        assert_eq!(winget[1].id, "Parent.Package2");
        assert_eq!(winget[1].version, Some("2.0.0".to_string())); // Overwritten by child
        assert_eq!(winget[2].id, "Child.Package1");
    }

    #[test]
    fn test_merge_brew_packages() {
        let parent = Some(Packages {
            brew: Some(vec![
                BrewPackage {
                    name: "git".to_string(),
                    cask: false,
                    tap: None,
                },
                BrewPackage {
                    name: "node".to_string(),
                    cask: false,
                    tap: None,
                },
            ]),
            ..Default::default()
        });
        let child = Some(Packages {
            brew: Some(vec![
                BrewPackage {
                    name: "wget".to_string(),
                    cask: false,
                    tap: None,
                },
                BrewPackage {
                    name: "node".to_string(),
                    cask: false,
                    tap: Some("custom/tap".to_string()),
                },
            ]),
            ..Default::default()
        });

        let merged = merge_packages(parent, child).unwrap();
        let brew = merged.brew.unwrap();
        assert_eq!(brew.len(), 3);
        assert_eq!(brew[0].name, "git");
        assert_eq!(brew[1].name, "node");
        assert_eq!(brew[1].tap.as_deref(), Some("custom/tap")); // Overwritten by child
        assert_eq!(brew[2].name, "wget");
    }

    #[test]
    fn test_merge_apt_packages() {
        let parent = Some(Packages {
            apt: Some(vec![AptPackage {
                name: "git".to_string(),
                version: None,
            }]),
            ..Default::default()
        });
        let child = Some(Packages {
            apt: Some(vec![
                AptPackage {
                    name: "git".to_string(),
                    version: Some("2.40".to_string()),
                },
                AptPackage {
                    name: "curl".to_string(),
                    version: None,
                },
            ]),
            ..Default::default()
        });

        let merged = merge_packages(parent, child).unwrap();
        let apt = merged.apt.unwrap();
        assert_eq!(apt.len(), 2);
        assert_eq!(apt[0].name, "git");
        assert_eq!(apt[0].version.as_deref(), Some("2.40")); // Overwritten by child
        assert_eq!(apt[1].name, "curl");
    }

    #[test]
    fn test_merge_mixed_managers() {
        let parent = Some(Packages {
            winget: Some(vec![WingetPackage::new("Git.Git")]),
            brew: Some(vec![BrewPackage {
                name: "git".to_string(),
                cask: false,
                tap: None,
            }]),
            apt: None,
        });
        let child = Some(Packages {
            winget: None,
            brew: None,
            apt: Some(vec![AptPackage {
                name: "git".to_string(),
                version: None,
            }]),
        });

        let merged = merge_packages(parent, child).unwrap();
        assert!(merged.winget.is_some());
        assert!(merged.brew.is_some());
        assert!(merged.apt.is_some());
    }

    #[test]
    fn test_merge_files_no_conflict() {
        let parent = Some(vec![FileEntry::new("parent.txt", "~/parent.txt")]);
        let child = Some(vec![FileEntry::new("child.txt", "~/child.txt")]);

        let merged = merge_files(parent, child).unwrap();
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_merge_files_with_conflict() {
        let parent = Some(vec![FileEntry::new("config.txt", "~/config.txt")]);
        let child = Some(vec![FileEntry {
            source: "new-config.txt".to_string(),
            destination: "~/config.txt".to_string(), // Same destination
            backup: false,
            permissions: None,
            template: true,
        }]);

        let merged = merge_files(parent, child).unwrap();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "new-config.txt");
        assert!(!merged[0].backup);
        assert!(merged[0].template);
    }

    #[test]
    fn test_merge_environment_variables() {
        let parent = Some(Environment {
            variables: Some(vec![
                EnvVariable::new("VAR1", "parent_value1"),
                EnvVariable::new("VAR2", "parent_value2"),
            ]),
            path_additions: None,
        });
        let child = Some(Environment {
            variables: Some(vec![
                EnvVariable::new("VAR2", "child_value2"), // Override
                EnvVariable::new("VAR3", "child_value3"),
            ]),
            path_additions: None,
        });

        let merged = merge_environment(parent, child).unwrap();
        let vars = merged.variables.unwrap();

        assert_eq!(vars.len(), 3);
        assert_eq!(
            vars.iter().find(|v| v.name == "VAR1").unwrap().value,
            "parent_value1"
        );
        assert_eq!(
            vars.iter().find(|v| v.name == "VAR2").unwrap().value,
            "child_value2"
        );
        assert_eq!(
            vars.iter().find(|v| v.name == "VAR3").unwrap().value,
            "child_value3"
        );
    }

    #[test]
    fn test_merge_path_additions() {
        let parent = Some(Environment {
            variables: None,
            path_additions: Some(vec!["~/parent/bin".to_string(), "~/shared/bin".to_string()]),
        });
        let child = Some(Environment {
            variables: None,
            path_additions: Some(vec![
                "~/child/bin".to_string(),
                "~/shared/bin".to_string(), // Duplicate
            ]),
        });

        let merged = merge_environment(parent, child).unwrap();
        let paths = merged.path_additions.unwrap();

        assert_eq!(paths.len(), 3); // Duplicate removed
        assert!(paths.contains(&"~/parent/bin".to_string()));
        assert!(paths.contains(&"~/shared/bin".to_string()));
        assert!(paths.contains(&"~/child/bin".to_string()));
    }

    #[test]
    fn test_extends_field_removed_after_merge() {
        let parent = Workload {
            name: "parent".to_string(),
            extends: Some(vec!["grandparent".to_string()]),
            ..Workload::empty()
        };
        let child = Workload {
            name: "child".to_string(),
            extends: Some(vec!["parent".to_string()]),
            ..Workload::empty()
        };

        let merged = merge_workloads(parent, child);
        assert!(merged.extends.is_none());
    }

    #[test]
    fn test_merge_scripts_order() {
        use crate::config::workload::ScriptEntry;

        let parent = Some(Scripts {
            pre_install: Some(vec![ScriptEntry::new("parent-pre.ps1")]),
            post_install: Some(vec![ScriptEntry::new("parent-post.ps1")]),
            health_check: None,
        });
        let child = Some(Scripts {
            pre_install: Some(vec![ScriptEntry::new("child-pre.ps1")]),
            post_install: Some(vec![ScriptEntry::new("child-post.ps1")]),
            health_check: None,
        });

        let merged = merge_scripts(parent, child).unwrap();

        // Pre-install: parent first, then child
        let pre = merged.pre_install.unwrap();
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0].path, "parent-pre.ps1");
        assert_eq!(pre[1].path, "child-pre.ps1");

        // Post-install: parent first, then child
        let post = merged.post_install.unwrap();
        assert_eq!(post.len(), 2);
        assert_eq!(post[0].path, "parent-post.ps1");
        assert_eq!(post[1].path, "child-post.ps1");
    }

    #[test]
    fn test_inheritance_error_display() {
        let err = InheritanceError::CircularDependency {
            chain: "a -> b -> a".to_string(),
        };
        assert!(err.to_string().contains("Circular dependency"));
        assert!(err.to_string().contains("a -> b -> a"));

        let err = InheritanceError::ParentNotFound {
            parent: "missing-parent".to_string(),
        };
        assert!(err.to_string().contains("missing-parent"));
        assert!(err.to_string().contains("not found"));
    }
}
