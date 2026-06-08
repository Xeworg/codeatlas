//! Application services — hexagonal architecture application layer (wave 1).
//!
//! Services in this module orchestrate use-cases by delegating to canonical
//! ports (`ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AppStatePort`).
//! They do NOT instantiate infrastructure directly; infrastructure is injected
//! at construction time from the composition root (`src-tauri/src/lib.rs`).
//!
//! # Design (AD-3, AD-5)
//!
//! ```text
//! Tauri command shim
//!   -> ScanService / GraphService / WorkspaceService / AnalysisService
//!     -> Ports
//!       -> ProjectRepository adapters / AppState adapter
//! ```
//!
//! # Services
//! - [`ScanService`] — scan, reopen, and status for a project
//! - [`GraphService`] — graph, node, outline, and search orchestration
//! - [`WorkspaceService`] — workspace, snapshot, annotation, health, and executive operations

pub mod analysis_service;
pub mod graph_service;
pub mod scan_service;
pub mod workspace_service;

pub use analysis_service::AnalysisService;
pub use analysis_service::{
    ArchitectureDetectionResponse, ExportMetadata, ExportPayloadResponse, GraphInsightsResponse,
    ImpactAnalysisResponse,
};
pub use graph_service::GraphService;
pub use scan_service::ScanService;
pub use workspace_service::{
    AnnotationResponse, C4ViewResponse, ExecutiveSummaryResponse, HealthRecordResponse,
    HealthTimelineResponse, HotspotItem, SnapshotDiffResponse, SnapshotResponse,
    WorkspaceProjectResponse, WorkspaceResponse, WorkspaceService,
};
