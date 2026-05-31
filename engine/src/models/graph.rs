//! Graph domain models

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum NodeType {
    Component,
    Route,
    Service,
    Repository,
    Model,
    Util,
    Config,
    Test,
    External,
    #[default]
    Unknown,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub path: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub symbol_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub imports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub project_id: String,
    pub generated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_node_with_position_serializes() {
        let node = GraphNode {
            id: "node-1".into(),
            label: "UserService.ts".into(),
            path: "src/services/UserService.ts".into(),
            node_type: NodeType::Service,
            symbol_count: 5,
            position: Some(Position { x: 100.0, y: 200.0 }),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("service"));
        assert!(json.contains("100"));
    }

    #[test]
    fn graph_edge_identifies_import_relationship() {
        let edge = GraphEdge {
            id: "edge-1".into(),
            source: "UserController.ts".into(),
            target: "UserService.ts".into(),
            imports: vec!["UserService".into()],
        };

        let json = serde_json::to_string(&edge).unwrap();
        let parsed: GraphEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.source, "UserController.ts");
        assert_eq!(parsed.target, "UserService.ts");
        assert_eq!(parsed.imports.len(), 1);
    }
}
