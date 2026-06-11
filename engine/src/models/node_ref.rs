//! NodeRef — DTO for graph dependency queries.
//!
//! Returned by `GraphService::get_dependencies` and `get_dependents`.
//! Represents a node that is a dependency (or dependent) of another node.

use serde::{Deserialize, Serialize};

/// A node reference returned by dependency queries.
/// Represents a node that the queried node depends on (outgoing edges)
/// or a node that depends on the queried node (incoming edges).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRef {
    /// Unique node identifier (file UUID).
    pub id: String,
    /// Source file path of this node.
    pub source: String,
    /// Target module/path this node represents.
    pub target: String,
    /// Import statement that creates this dependency.
    pub imports: Vec<String>,
}

impl NodeRef {
    /// Create a new NodeRef.
    pub fn new(id: String, source: String, target: String, imports: Vec<String>) -> Self {
        Self {
            id,
            source,
            target,
            imports,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ref_serializes_to_camel_case_json() {
        let node = NodeRef::new(
            "node-1".into(),
            "/src/a.ts".into(),
            "./b".into(),
            vec!["import { b } from './b'".into()],
        );
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"id\":\"node-1\""));
        assert!(json.contains("\"source\":\"/src/a.ts\""));
        assert!(json.contains("\"target\":\"./b\""));
        assert!(json.contains("\"imports\""));
    }
}