//! Graph builder — constructs dependency graph from file metadata.

use std::collections::HashMap;

use crate::models::{FileInfo, GraphData, GraphEdge, GraphNode, ImportInfo, NodeType};
use crate::Result;

pub struct GraphBuilder {
    resolver: crate::graph::PathResolver,
}

impl GraphBuilder {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            resolver: crate::graph::PathResolver::new(root),
        }
    }

    /// Build a dependency graph from scanned files.
    pub fn build(&self, files: &[FileInfo], imports: &[ImportInfo]) -> Result<GraphData> {
        let mut nodes: Vec<GraphNode> = Vec::new();
        let mut edges: Vec<GraphEdge> = Vec::new();
        let mut file_id_to_path: HashMap<String, String> = HashMap::new();
        let mut symbol_counts: HashMap<String, usize> = HashMap::new();

        // Build file index
        for file in files {
            let node_type = infer_node_type(&file.path, &file.symbols);
            let label = file.name.clone();
            let path = file.path.clone();

            file_id_to_path.insert(file.id.clone(), path.clone());
            symbol_counts.insert(file.id.clone(), file.symbols.len());

            nodes.push(GraphNode {
                id: file.id.clone(),
                label,
                path,
                node_type,
                symbol_count: file.symbols.len(),
                position: None,
            });
        }

        // Build edges from imports
        for import in imports {
            let _target_path = import
                .target_file_id
                .as_ref()
                .and_then(|id| file_id_to_path.get(id))
                .cloned()
                .or_else(|| import.target_module.clone());

            let target = if import.target_file_id.is_some() {
                import.target_file_id.clone()
            } else if import.target_module.as_ref().is_some() {
                // External module
                import.target_module.clone()
            } else {
                None
            };

            let (source, target_id) = (import.source_file_id.clone(), target);

            if let Some(t) = target_id {
                let target_node_id = if import.target_file_id.is_some() {
                    t.clone()
                } else {
                    // External: look for a matching node or create placeholder
                    t.clone()
                };

                edges.push(GraphEdge {
                    id: uuid::Uuid::new_v4().to_string(),
                    source,
                    target: target_node_id,
                    imports: import.imports.clone(),
                });
            }
        }

        let generated_at = chrono::Utc::now().to_rfc3339();
        let project_id = files
            .first()
            .map(|f| f.path.split('/').next().unwrap_or("unknown").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(GraphData {
            nodes,
            edges,
            project_id,
            generated_at,
        })
    }
}

/// Infer node type from file path and symbols.
fn infer_node_type(path: &str, symbols: &[crate::models::SymbolInfo]) -> NodeType {
    let lower = path.to_lowercase();
    let has_symbols = !symbols.is_empty();

    if lower.contains("/components/") || lower.ends_with(".tsx") && has_symbols {
        NodeType::Component
    } else if lower.contains("/routes/")
        || lower.contains("/pages/")
        || lower.ends_with(".route.ts")
    {
        NodeType::Route
    } else if lower.contains("/services/") || lower.contains("/api/") {
        NodeType::Service
    } else if lower.contains("/repositories/") || lower.contains("/dao/") || lower.contains("/dal/")
    {
        NodeType::Repository
    } else if lower.contains("/models/")
        || lower.contains("/types/")
        || lower.contains("/interfaces/")
    {
        NodeType::Model
    } else if lower.contains("/utils/") || lower.contains("/helpers/") || lower.contains("/lib/") {
        NodeType::Util
    } else if lower.contains("/config/")
        || lower.ends_with(".config.ts")
        || lower.ends_with(".config.js")
    {
        NodeType::Config
    } else if lower.contains("/test") || lower.contains(".spec.") || lower.ends_with("_test.rs") {
        NodeType::Test
    } else if lower.contains("node_modules") {
        NodeType::External
    } else {
        NodeType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SymbolInfo, SymbolKind};

    #[test]
    fn build_graph_with_three_files() {
        let files = vec![
            FileInfo {
                id: "f1".into(),
                path: "src/UserController.ts".into(),
                name: "UserController.ts".into(),
                extension: "ts".into(),
                symbols: vec![],
                lines: 50,
            },
            FileInfo {
                id: "f2".into(),
                path: "src/UserService.ts".into(),
                name: "UserService.ts".into(),
                extension: "ts".into(),
                symbols: vec![SymbolInfo {
                    id: "s1".into(),
                    name: "UserService".into(),
                    kind: SymbolKind::Class,
                    file_id: "f2".into(),
                    line_start: 1,
                    line_end: 30,
                    exports: true,
                }],
                lines: 30,
            },
            FileInfo {
                id: "f3".into(),
                path: "src/UserRepository.ts".into(),
                name: "UserRepository.ts".into(),
                extension: "ts".into(),
                symbols: vec![],
                lines: 20,
            },
        ];

        let imports = vec![
            ImportInfo {
                id: "i1".into(),
                source_file_id: "f1".into(),
                target_file_id: Some("f2".into()),
                target_module: None,
                imports: vec!["UserService".into()],
                is_default: false,
                is_type: false,
            },
            ImportInfo {
                id: "i2".into(),
                source_file_id: "f2".into(),
                target_file_id: Some("f3".into()),
                target_module: None,
                imports: vec!["UserRepository".into()],
                is_default: false,
                is_type: false,
            },
        ];

        let builder = GraphBuilder::new("/project");
        let graph = builder.build(&files, &imports).unwrap();

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].source, "f1");
        assert_eq!(graph.edges[0].target, "f2");
    }

    #[test]
    fn infer_node_type_from_path() {
        let files = vec![
            FileInfo {
                id: "c1".into(),
                path: "src/components/Button.tsx".into(),
                name: "Button.tsx".into(),
                extension: "tsx".into(),
                symbols: vec![SymbolInfo {
                    id: "s1".into(),
                    name: "Button".into(),
                    kind: SymbolKind::Function,
                    file_id: "c1".into(),
                    line_start: 1,
                    line_end: 20,
                    exports: true,
                }],
                lines: 20,
            },
            FileInfo {
                id: "sv1".into(),
                path: "src/services/AuthService.ts".into(),
                name: "AuthService.ts".into(),
                extension: "ts".into(),
                symbols: vec![],
                lines: 100,
            },
        ];

        let builder = GraphBuilder::new("/project");
        let graph = builder.build(&files, &[]).unwrap();

        let button_node = graph.nodes.iter().find(|n| n.id == "c1").unwrap();
        assert_eq!(button_node.node_type, NodeType::Component);

        let auth_node = graph.nodes.iter().find(|n| n.id == "sv1").unwrap();
        assert_eq!(auth_node.node_type, NodeType::Service);
    }
}
