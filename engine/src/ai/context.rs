//! Context builder — prepares compact context windows for AI prompts.

use crate::models::{GraphData, OutlineItem};

const MAX_CONTEXT_BYTES: usize = 8 * 1024; // 8KB cap
const MAX_DEPS: usize = 5;
const MAX_DEPENDENTS: usize = 3;

pub(crate) struct ContextBuilder;

impl ContextBuilder {
    /// Extract source lines for a specific line range.
    /// Returns the content between line_start and line_end (inclusive, 1-indexed).
    /// Falls back to an empty string if range is invalid or content is too short.
    pub fn extract_range(source: &str, line_start: u32, line_end: u32) -> String {
        if line_end < line_start || line_start == 0 {
            return String::new();
        }
        let mut current_line = 1u32;
        let mut start_byte = None;
        let mut end_byte = None;
        let bytes = source.as_bytes();

        for (i, b) in bytes.iter().enumerate() {
            // Line starts at index 0 or immediately after a newline
            let at_line_start = i == 0 || bytes[i - 1] == b'\n';

            // Record start: first byte of line_start
            if at_line_start && current_line >= line_start && start_byte.is_none() {
                start_byte = Some(i);
            }

            if *b == b'\n' {
                // Record end: byte AFTER the newline that ends line_end
                if current_line == line_end {
                    end_byte = Some(i + 1);
                    break;
                }
                current_line += 1;
            }
        }

        // If we reached EOF before line_end, use end of source
        if end_byte.is_none() {
            end_byte = Some(bytes.len());
        }

        match (start_byte, end_byte) {
            (Some(s), Some(e)) => String::from_utf8_lossy(&bytes[s..e]).to_string(),
            _ => String::new(),
        }
    }

    /// Build a compact context for node explanation using outline semantic data.
    /// When outline is available, produces a structured summary instead of raw truncation.
    /// Falls back to `build_node_context` behavior when outline is empty or unavailable.
    pub fn build_node_context_with_outline(
        file_content: &str,
        file_path: &str,
        graph: &GraphData,
        node_id: &str,
        outline: &[OutlineItem],
    ) -> String {
        let node = graph.nodes.iter().find(|n| n.id == node_id);

        // Collect deps and dependents (same logic as fallback)
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

        // Build node info
        let node_info = if let Some(n) = node {
            format!(
                "**Archivo:** {}\n**Tipo:** {:?}\n**Símbolos:** {}",
                n.path, n.node_type, n.symbol_count
            )
        } else {
            format!("**Archivo:** {}", file_path)
        };

        // Build outline summary (bounded to avoid bloating context)
        const MAX_OUTLINE_ITEMS: usize = 80;
        let outline_text = Self::render_outline_summary(outline, MAX_OUTLINE_ITEMS);

        // Fallback code excerpt (first half of cap) when outline is empty
        let fallback_code = if outline.is_empty() {
            Self::truncate_to_bytes(file_content, MAX_CONTEXT_BYTES / 2)
        } else {
            String::new() // outline provides structure; code excerpt can be added via extract_range later
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

        // Compose final context
        let mut context = node_info.clone();

        if !outline_text.is_empty() {
            context.push_str("\n\n**Outline:**\n");
            context.push_str(&outline_text);
        }

        context.push_str(&format!(
            "\n\n**Dependencias ({}):**\n{}\n\n**Dependientes ({}):**\n{}",
            deps.len(),
            deps_text,
            dependents.len(),
            dependents_text
        ));

        if !fallback_code.is_empty() {
            context.push_str("\n\n**Codigo (primeras lineas):**\n```\n");
            context.push_str(&fallback_code);
            context.push_str("\n```");
        }

        // Enforce total byte cap
        Self::truncate_to_bytes(&context, MAX_CONTEXT_BYTES)
    }

    /// Render a depth-first outline summary as plain text, limited to max_items.
    fn render_outline_summary(items: &[OutlineItem], max_items: usize) -> String {
        let mut output = String::new();
        let mut count = 0usize;

        fn render_item(
            item: &OutlineItem,
            depth: usize,
            output: &mut String,
            count: &mut usize,
            max: usize,
        ) {
            if *count >= max {
                output.push_str("(...mas simbolos...)\n");
                return;
            }
            let indent = "  ".repeat(depth);
            let kind_str = format!("[{:?}]", item.kind);
            output.push_str(&format!(
                "{}{} {} (lines {}-{})\n",
                indent, kind_str, item.name, item.line_start, item.line_end
            ));
            *count += 1;
            for child in &item.children {
                render_item(child, depth + 1, output, count, max);
            }
        }

        for item in items {
            render_item(item, 0, &mut output, &mut count, max_items);
        }

        output
    }

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
    use crate::models::{GraphEdge, GraphNode, NodeType, OutlineItemKind};

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

    // ──────────────────────────────────────────────────────────────
    // T4.1 RED tests — semantic context tests (will fail until implement)
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn outline_semantic_context_includes_hierarchy() {
        let graph = make_graph_with_deps(0, 0);
        let outline = vec![
            OutlineItem {
                id: "outline:file-1:class:1:30:UserService".into(),
                file_id: "file-1".into(),
                name: "UserService".into(),
                kind: OutlineItemKind::Class,
                line_start: 1,
                line_end: 30,
                column_start: Some(0),
                column_end: Some(0),
                children: vec![OutlineItem {
                    id: "outline:file-1:method:5:8:getUser".into(),
                    file_id: "file-1".into(),
                    name: "getUser".into(),
                    kind: OutlineItemKind::Method,
                    line_start: 5,
                    line_end: 8,
                    column_start: None,
                    column_end: None,
                    children: vec![],
                }],
            },
            OutlineItem {
                id: "outline:file-1:function:32:40:parseData".into(),
                file_id: "file-1".into(),
                name: "parseData".into(),
                kind: OutlineItemKind::Function,
                line_start: 32,
                line_end: 40,
                column_start: None,
                column_end: None,
                children: vec![],
            },
        ];

        let context = ContextBuilder::build_node_context_with_outline(
            "// some source",
            "src/services/UserService.ts",
            &graph,
            "file-1",
            &outline,
        );

        // Outline hierarchy should appear
        assert!(context.contains("UserService"), "class name should appear");
        assert!(context.contains("getUser"), "method name should appear");
        assert!(context.contains("parseData"), "function name should appear");
        // Outline section should be present
        assert!(context.contains("Outline"), "outline section should appear");
        // Node metadata should still be present
        assert!(context.contains("src/services/UserService.ts"));
    }

    #[test]
    fn outline_semantic_context_respects_byte_cap() {
        let graph = make_graph_with_deps(0, 0);
        // Very large outline that would exceed 8KB if fully rendered
        let mut outline = vec![];
        for i in 0..200 {
            outline.push(OutlineItem {
                id: format!("item-{}", i),
                file_id: "file-1".into(),
                name: format!("VeryLongSymbolName{}", i),
                kind: OutlineItemKind::Function,
                line_start: i * 10,
                line_end: i * 10 + 8,
                column_start: None,
                column_end: None,
                children: vec![],
            });
        }

        let context = ContextBuilder::build_node_context_with_outline(
            "", "large.ts", &graph, "file-1", &outline,
        );

        // Context must stay within MAX_CONTEXT_BYTES
        assert!(
            context.len() <= MAX_CONTEXT_BYTES,
            "Context {} bytes exceeds cap {}",
            context.len(),
            MAX_CONTEXT_BYTES
        );
    }

    #[test]
    fn outline_semantic_context_falls_back_when_empty() {
        let graph = make_graph_with_deps(0, 0);
        let source = "fn main() {\n    println!(\"hello\");\n}";

        let context = ContextBuilder::build_node_context_with_outline(
            source,
            "src/main.rs",
            &graph,
            "main",
            &[], // empty outline
        );

        // Falls back to source truncation when no outline
        assert!(
            context.contains("Codigo") || context.contains("fn main"),
            "Should fall back to source code excerpt"
        );
        // Outline section should NOT appear when outline is empty
        assert!(
            !context.contains("**Outline:**"),
            "No outline section for empty outline"
        );
    }

    #[test]
    fn extract_range_respects_boundaries() {
        let source = "line1\nline2\nline3\nline4\nline5\n";

        // Extract lines 2-4
        let extracted = ContextBuilder::extract_range(source, 2, 4);
        assert_eq!(extracted, "line2\nline3\nline4\n");

        // Extract single line
        let single = ContextBuilder::extract_range(source, 3, 3);
        assert_eq!(single, "line3\n");

        // Invalid range (end < start) returns empty
        let invalid = ContextBuilder::extract_range(source, 5, 2);
        assert_eq!(invalid, "");

        // Zero start returns empty
        let zero = ContextBuilder::extract_range(source, 0, 3);
        assert_eq!(zero, "");

        // Past end returns up to EOF
        let past_end = ContextBuilder::extract_range(source, 3, 100);
        assert!(past_end.contains("line3"));
    }

    #[test]
    fn outline_context_shows_dependencies_and_dependents() {
        let graph = make_graph_with_deps(3, 2);
        let outline = vec![OutlineItem {
            id: "file-1:class:1:10:X".into(),
            file_id: "file-1".into(),
            name: "X".into(),
            kind: OutlineItemKind::Class,
            line_start: 1,
            line_end: 10,
            column_start: None,
            column_end: None,
            children: vec![],
        }];

        let context = ContextBuilder::build_node_context_with_outline(
            "// code", "test.ts", &graph, "main", &outline,
        );

        // Should include first 5 deps and first 3 dependents
        assert!(context.contains("dep0"), "dep0 should appear");
        assert!(context.contains("dependent0"), "dependent0 should appear");
        assert!(
            context.contains("Dependencias (3)"),
            "deps count should appear"
        );
        assert!(
            context.contains("Dependientes (2)"),
            "dependents count should appear"
        );
    }
}
