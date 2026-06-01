//! Context builder — prepares compact context windows for AI prompts.

use crate::models::GraphData;

const MAX_CONTEXT_BYTES: usize = 8 * 1024; // 8KB cap
const MAX_DEPS: usize = 5;
const MAX_DEPENDENTS: usize = 3;

pub struct ContextBuilder;

impl ContextBuilder {
    /// Build a compact context for node explanation.
    pub fn build_node_context(
        file_content: &str,
        file_path: &str,
        graph: &GraphData,
        node_id: &str,
    ) -> String {
        let node = graph.nodes.iter().find(|n| n.id == node_id);

        let deps: Vec<String> = graph
            .edges
            .iter()
            .filter(|e| e.source == node_id)
            .take(MAX_DEPS)
            .filter_map(|e| {
                graph
                    .nodes
                    .iter()
                    .find(|n| n.id == e.target)
                    .map(|n| format!("- **{}** ({})", n.label, n.path))
            })
            .collect();

        let dependents: Vec<String> = graph
            .edges
            .iter()
            .filter(|e| e.target == node_id)
            .take(MAX_DEPENDENTS)
            .filter_map(|e| {
                graph
                    .nodes
                    .iter()
                    .find(|n| n.id == e.source)
                    .map(|n| format!("- **{}** ({})", n.label, n.path))
            })
            .collect();

        let truncated_content = Self::truncate_to_bytes(file_content, MAX_CONTEXT_BYTES / 2);

        let node_info = if let Some(n) = node {
            format!(
                "**Archivo:** {}\n**Tipo:** {:?}\n**Símbolos:** {}",
                n.path, n.node_type, n.symbol_count
            )
        } else {
            format!("**Archivo:** {}", file_path)
        };

        let deps_text = if deps.is_empty() {
            "Ninguna".to_string()
        } else {
            deps.join("\n")
        };

        let dependents_text = if dependents.is_empty() {
            "Ninguno".to_string()
        } else {
            dependents.join("\n")
        };

        format!(
            "{}\n\n**Dependencias ({}):**\n{}\n\n**Dependientes ({}):**\n{}\n\n**Codigo (primeras lineas):**\n```\n{}\n```",
            node_info,
            deps.len(),
            deps_text,
            dependents.len(),
            dependents_text,
            truncated_content
        )
    }

    /// Build context for chat with project-wide questions.
    pub fn build_chat_context(
        files: &[(&str, &str)], // (path, content)
        graph: &GraphData,
        question: &str,
    ) -> String {
        let mut context = format!("**Pregunta:** {}\n\n", question);

        context.push_str("**Estructura del proyecto:**\n");
        for node in &graph.nodes {
            context.push_str(&format!("- {} [{:?}]\n", node.label, node.node_type));
            if context.len() > MAX_CONTEXT_BYTES / 2 {
                context.push_str("(mas archivos...)\n");
                break;
            }
        }

        // Most relevant files based on question keywords
        let question_lower = question.to_lowercase();
        let keywords: Vec<&str> = question_lower.split_whitespace().collect();

        context.push_str("\n**Archivos relevantes:**\n");
        let relevant: Vec<_> = files
            .iter()
            .filter(|(path, _)| keywords.iter().any(|kw| path.to_lowercase().contains(kw)))
            .take(3)
            .collect();

        for (path, content) in relevant {
            let truncated = Self::truncate_to_bytes(content, 500);
            context.push_str(&format!("\n**{}:**\n```\n{}\n```\n", path, truncated));
        }

        context
    }

    fn truncate_to_bytes(s: &str, max_bytes: usize) -> String {
        let mut bytes = 0;
        s.chars()
            .take_while(|c| {
                bytes += c.len_utf8();
                bytes <= max_bytes
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GraphEdge, GraphNode, NodeType};

    fn make_graph_with_deps(num_deps: usize, num_dependents: usize) -> GraphData {
        let mut nodes = vec![GraphNode {
            id: "main".into(),
            label: "main.rs".into(),
            path: "src/main.rs".into(),
            node_type: NodeType::Unknown,
            symbol_count: 1,
            position: None,
        }];

        let mut edges: Vec<GraphEdge> = vec![];

        // Add dependencies (edges from main → dep)
        for i in 0..num_deps {
            let id = format!("dep{}", i);
            nodes.push(GraphNode {
                id: id.clone(),
                label: format!("dep{}.rs", i),
                path: format!("src/dep{}.rs", i),
                node_type: NodeType::Unknown,
                symbol_count: 1,
                position: None,
            });
            edges.push(GraphEdge {
                id: format!("e-dep-{}", i),
                source: "main".into(),
                target: id,
                imports: vec![],
            });
        }

        // Add dependents (edges from dep → main)
        for i in 0..num_dependents {
            let id = format!("dependent{}", i);
            nodes.push(GraphNode {
                id: id.clone(),
                label: format!("dependent{}.rs", i),
                path: format!("src/dependent{}.rs", i),
                node_type: NodeType::Unknown,
                symbol_count: 1,
                position: None,
            });
            edges.push(GraphEdge {
                id: format!("e-dnt-{}", i),
                source: id,
                target: "main".into(),
                imports: vec![],
            });
        }

        GraphData {
            nodes,
            edges,
            project_id: "test".into(),
            generated_at: "now".into(),
        }
    }

    #[test]
    fn context_respects_8kb_limit() {
        let large_code = "x".repeat(20_000);
        let graph = make_graph_with_deps(0, 0);
        let context = ContextBuilder::build_node_context(&large_code, "test.rs", &graph, "main");
        assert!(
            context.len() <= MAX_CONTEXT_BYTES,
            "Context {} bytes exceeds limit {}",
            context.len(),
            MAX_CONTEXT_BYTES
        );
    }

    #[test]
    fn context_includes_top_5_deps() {
        let graph = make_graph_with_deps(10, 0);
        let context = ContextBuilder::build_node_context("fn main() {}", "main.rs", &graph, "main");

        // Only the first 5 deps should appear in the context
        assert!(context.contains("dep0"), "dep0 should appear");
        assert!(context.contains("dep4"), "dep4 should appear (5th dep)");
        assert!(
            !context.contains("dep5"),
            "dep5 should NOT appear (6th dep)"
        );
        assert!(!context.contains("dep9"), "dep9 should NOT appear");

        let dep_count = (0..10)
            .filter(|i| context.contains(&format!("dep{}", i)))
            .count();
        assert_eq!(dep_count, MAX_DEPS);
    }

    #[test]
    fn context_includes_top_3_dependents() {
        let graph = make_graph_with_deps(0, 10);
        let context = ContextBuilder::build_node_context("fn main() {}", "main.rs", &graph, "main");

        assert!(context.contains("dependent0"), "dependent0 should appear");
        assert!(
            context.contains("dependent2"),
            "dependent2 should appear (3rd)"
        );
        assert!(
            !context.contains("dependent3"),
            "dependent3 should NOT appear (4th)"
        );

        let dep_count = (0..10)
            .filter(|i| context.contains(&format!("dependent{}", i)))
            .count();
        assert_eq!(dep_count, MAX_DEPENDENTS);
    }

    #[test]
    fn context_includes_node_metadata() {
        let graph = make_graph_with_deps(0, 0);
        let context = ContextBuilder::build_node_context("fn main() {}", "main.rs", &graph, "main");

        assert!(context.contains("src/main.rs"), "File path should appear");
        assert!(
            context.contains("Dependencias"),
            "Deps section header should appear"
        );
        assert!(
            context.contains("Dependientes"),
            "Dependents section header should appear"
        );
        assert!(context.contains("Codigo"), "Code section should appear");
    }
}
