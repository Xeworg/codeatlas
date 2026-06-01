//! File walker with directory exclusions.

use std::path::Path;

use ignore::WalkBuilder;

const EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    "target",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "coverage",
    "__pycache__",
];

const SUPPORTED_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "rs", "json"];

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: String,
    pub relative_path: String,
    pub extension: String,
    pub size_bytes: u64,
}

pub struct FileWalker {
    root: String,
}

impl FileWalker {
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }

    pub fn discover(&self) -> Vec<DiscoveredFile> {
        let root = Path::new(&self.root);
        let root_str = self.root.clone();

        WalkBuilder::new(root)
            .hidden(true)
            .filter_entry(|entry| {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy();
                    !EXCLUDED_DIRS.contains(&name.as_ref())
                } else {
                    true
                }
            })
            .build()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                if !entry.file_type()?.is_file() {
                    return None;
                }

                let path = entry.path();
                let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

                if !SUPPORTED_EXTENSIONS.contains(&extension) {
                    return None;
                }

                let relative = path
                    .strip_prefix(&root_str)
                    .ok()?
                    .to_string_lossy()
                    .into_owned();

                let size = entry.metadata().ok()?.len();

                Some(DiscoveredFile {
                    path: path.to_string_lossy().into_owned(),
                    relative_path: relative,
                    extension: extension.to_string(),
                    size_bytes: size,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn walker_excludes_node_modules() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create file inside node_modules (should be excluded)
        std::fs::create_dir_all(root.join("node_modules/some/pkg")).ok();
        std::fs::write(
            root.join("node_modules/some/pkg/index.js"),
            "export const x = 1;",
        )
        .ok();

        // Create regular TypeScript files
        std::fs::write(root.join("index.ts"), "export const y = 2;").ok();
        std::fs::create_dir_all(root.join("src")).ok();
        std::fs::write(root.join("src/main.ts"), "export const z = 3;").ok();

        let walker = FileWalker::new(root.to_string_lossy().as_ref());
        let files = walker.discover();

        let paths: Vec<_> = files.iter().map(|f| f.relative_path.clone()).collect();

        // Should find TypeScript files
        assert!(
            paths.iter().any(|p| p.ends_with(".ts")),
            "Expected at least one .ts file, found: {:?}",
            paths
        );
        // Should NOT include anything from node_modules
        assert!(
            !paths.iter().any(|p| p.contains("node_modules")),
            "node_modules should be excluded, found: {:?}",
            paths
        );
    }

    #[test]
    fn walker_only_finds_supported_extensions() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        std::fs::write(root.join("Main.ts"), "const a = 1;").ok();
        std::fs::write(root.join("lib.rs"), "fn main() {}").ok();
        std::fs::write(root.join("config.json"), "{}").ok();
        std::fs::write(root.join("README.md"), "# No").ok(); // unsupported

        let walker = FileWalker::new(root.to_string_lossy().as_ref());
        let extensions: Vec<_> = walker
            .discover()
            .iter()
            .map(|f| f.extension.clone())
            .collect();

        assert!(extensions.contains(&"ts".into()));
        assert!(extensions.contains(&"rs".into()));
        assert!(extensions.contains(&"json".into()));
        assert!(!extensions.contains(&"md".into()));
    }
}
