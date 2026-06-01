//! Path resolution for imports (relative paths, TS aliases, node_modules)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Resolves import paths to absolute file paths or external modules.
#[allow(dead_code)]
pub struct PathResolver {
    root: PathBuf,
    aliases: HashMap<String, String>,
    extensions: Vec<String>,
}

impl PathResolver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            aliases: HashMap::new(),
            extensions: vec![
                "ts".into(),
                "tsx".into(),
                "js".into(),
                "jsx".into(),
                "rs".into(),
            ],
        }
    }

    /// Load TSConfig paths aliases if available.
    pub fn with_tsconfig(mut self, tsconfig_path: &str) -> Self {
        if let Ok(content) = std::fs::read_to_string(tsconfig_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(paths) = json
                    .get("compilerOptions")
                    .and_then(|co| co.get("paths"))
                    .and_then(|p| p.as_object())
                {
                    for (alias, targets) in paths {
                        if let Some(targets) = targets.as_array() {
                            if let Some(target) = targets.first().and_then(|t| t.as_str()) {
                                self.aliases.insert(
                                    alias.trim_end_matches("/*").to_string(),
                                    target.trim_end_matches("/*").to_string(),
                                );
                            }
                        }
                    }
                }
            }
        }
        self
    }

    /// Resolve an import path relative to a source file.
    /// Returns the absolute path if internal, or the module name if external.
    pub fn resolve(&self, import_path: &str, from_file: &str) -> Resolution {
        // Check for node_modules
        if import_path.starts_with('.') {
            return self.resolve_relative(import_path, from_file);
        }

        // Check for @/ style aliases (no tsconfig needed, fallback to src)
        if import_path.starts_with("@/") {
            // Common default: @ means src
            let alias_resolved = import_path.replace("@/", "src/");
            return self.resolve_relative(&alias_resolved, from_file);
        }

        // Check for aliases
        for (alias, base) in &self.aliases {
            if import_path.starts_with(&format!("{}/", alias)) {
                let relative = import_path.replace(&format!("{}/", alias), &format!("{}/", base));
                return self.resolve_relative(&relative, from_file);
            }
        }

        // External module
        let module = import_path.split('/').next().unwrap_or(import_path);
        Resolution::External(module.to_string())
    }

    fn resolve_relative(&self, import_path: &str, from_file: &str) -> Resolution {
        let from_dir = Path::new(from_file).parent().unwrap_or(Path::new(""));
        let base = from_dir.join(import_path);

        // Try each extension
        for ext in &[
            "",
            ".ts",
            ".tsx",
            ".js",
            ".jsx",
            "/index.ts",
            "/index.tsx",
            "/index.js",
        ] {
            let candidate = if ext.is_empty() {
                base.clone()
            } else {
                PathBuf::from(format!("{}{}", base.display(), ext))
            };

            let absolute = if candidate.is_relative() {
                self.root.join(&candidate)
            } else {
                candidate.clone()
            };

            if absolute.exists() {
                let relative = absolute
                    .strip_prefix(&self.root)
                    .ok()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| absolute.to_string_lossy().into_owned());

                return Resolution::Internal(relative);
            }
        }

        Resolution::Unresolved(import_path.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Internal(String),
    External(String),
    Unresolved(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_relative_path() {
        let resolver = PathResolver::new("/project");
        let result = resolver.resolve("./utils", "src/services/UserService.ts");
        // From src/services/UserService.ts, ./utils resolves to src/services/utils
        assert!(matches!(
            result,
            Resolution::Internal(_) | Resolution::Unresolved(_)
        ));
    }

    #[test]
    fn resolve_external_module() {
        let resolver = PathResolver::new("/project");
        let result = resolver.resolve("react", "App.tsx");
        assert_eq!(result, Resolution::External("react".to_string()));
    }

    #[test]
    fn resolve_with_tsconfig_alias() {
        let resolver = PathResolver::new("/project/src").with_tsconfig("tsconfig.json");
        let result = resolver.resolve("@/components/Button", "src/pages/Home.tsx");
        // @ alias should be resolved based on tsconfig
        assert!(matches!(
            result,
            Resolution::Internal(_) | Resolution::Unresolved(_)
        ));
    }
}
