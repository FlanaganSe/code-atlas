//! Cross-boundary contract tests.
//!
//! These tests verify that every type crossing the Rust -> TypeScript boundary
//! serializes to JSON with the correct field names and structure the frontend expects.
//! If a serde rename is accidentally changed or dropped, these tests catch it.

use codeatlas_core::graph::identity::{EdgeId, MaterializedKey};
use codeatlas_core::graph::types::*;
use codeatlas_core::health::compatibility::{CompatibilityDetail, CompatibilityReport, SupportStatus};
use codeatlas_core::health::graph_health::GraphHealth;
use codeatlas_core::profile::GraphProfile;
use codeatlas_core::scan::ScanPhase;

// ---------------------------------------------------------------------------
// NodeData
// ---------------------------------------------------------------------------

#[test]
fn contract_node_data() {
    let node = NodeData {
        materialized_key: MaterializedKey::new(Language::Rust, EntityKind::Package, "crates/core"),
        lineage_key: Some(LineageKey("abc-123".to_string())),
        label: "codeatlas-core".to_string(),
        kind: NodeKind::Package,
        language: Language::Rust,
        parent_key: None,
    };
    let json = serde_json::to_value(&node).unwrap();

    assert!(json.get("materializedKey").is_some(), "missing materializedKey");
    assert!(json.get("lineageKey").is_some(), "missing lineageKey");
    assert!(json.get("label").is_some(), "missing label");
    assert!(json.get("kind").is_some(), "missing kind");
    assert!(json.get("language").is_some(), "missing language");
    assert!(json.get("parentKey").is_some(), "missing parentKey");

    // No snake_case leaks
    assert!(json.get("materialized_key").is_none(), "snake_case leak: materialized_key");
    assert!(json.get("lineage_key").is_none(), "snake_case leak: lineage_key");
    assert!(json.get("parent_key").is_none(), "snake_case leak: parent_key");

    // Verify values
    assert_eq!(json["label"], "codeatlas-core");
    assert_eq!(json["language"], "rust");
    assert_eq!(json["kind"], "package");
    assert_eq!(json["parentKey"], serde_json::Value::Null);
}

// ---------------------------------------------------------------------------
// EdgeData
// ---------------------------------------------------------------------------

#[test]
fn contract_edge_data() {
    let source = MaterializedKey::new(Language::Rust, EntityKind::File, "src/lib.rs");
    let target = MaterializedKey::new(Language::Rust, EntityKind::File, "src/graph/mod.rs");

    let edge = EdgeData {
        edge_id: EdgeId::new(&source, &target, EdgeKind::Imports, EdgeCategory::Value),
        source_key: source,
        target_key: target,
        kind: EdgeKind::Imports,
        category: EdgeCategory::Value,
        confidence: Confidence::Syntactic,
        source_location: Some(SourceLocation {
            path: "src/lib.rs".into(),
            start_line: 1,
            end_line: 1,
        }),
        resolution_method: Some("tree-sitter".to_string()),
        overlay_status: OverlayStatus::None,
    };
    let json = serde_json::to_value(&edge).unwrap();

    assert!(json.get("edgeId").is_some(), "missing edgeId");
    assert!(json.get("sourceKey").is_some(), "missing sourceKey");
    assert!(json.get("targetKey").is_some(), "missing targetKey");
    assert!(json.get("kind").is_some(), "missing kind");
    assert!(json.get("category").is_some(), "missing category");
    assert!(json.get("confidence").is_some(), "missing confidence");
    assert!(json.get("sourceLocation").is_some(), "missing sourceLocation");
    assert!(json.get("resolutionMethod").is_some(), "missing resolutionMethod");
    assert!(json.get("overlayStatus").is_some(), "missing overlayStatus");

    // No snake_case leaks
    assert!(json.get("edge_id").is_none(), "snake_case leak: edge_id");
    assert!(json.get("source_key").is_none(), "snake_case leak: source_key");
    assert!(json.get("target_key").is_none(), "snake_case leak: target_key");
    assert!(json.get("source_location").is_none(), "snake_case leak: source_location");
    assert!(json.get("resolution_method").is_none(), "snake_case leak: resolution_method");
    assert!(json.get("overlay_status").is_none(), "snake_case leak: overlay_status");

    // Verify enum values are camelCase
    assert_eq!(json["kind"], "imports");
    assert_eq!(json["category"], "value");
    assert_eq!(json["confidence"], "syntactic");
}

// ---------------------------------------------------------------------------
// MaterializedKey
// ---------------------------------------------------------------------------

#[test]
fn contract_materialized_key() {
    let key = MaterializedKey::new(Language::TypeScript, EntityKind::Module, "src/graph");
    let json = serde_json::to_value(&key).unwrap();

    assert!(json.get("language").is_some(), "missing language");
    assert!(json.get("entityKind").is_some(), "missing entityKind");
    assert!(json.get("relativePath").is_some(), "missing relativePath");

    // No snake_case leaks
    assert!(json.get("entity_kind").is_none(), "snake_case leak: entity_kind");
    assert!(json.get("relative_path").is_none(), "snake_case leak: relative_path");

    assert_eq!(json["language"], "typescript");
    assert_eq!(json["entityKind"], "module");
    assert_eq!(json["relativePath"], "src/graph");
}

// ---------------------------------------------------------------------------
// ScanPhase
// ---------------------------------------------------------------------------

#[test]
fn contract_scan_phase() {
    assert_eq!(
        serde_json::to_value(ScanPhase::PackageTopology).unwrap(),
        "packageTopology"
    );
    assert_eq!(
        serde_json::to_value(ScanPhase::ModuleStructure).unwrap(),
        "moduleStructure"
    );
    assert_eq!(
        serde_json::to_value(ScanPhase::FileEdges).unwrap(),
        "fileEdges"
    );
}

// ---------------------------------------------------------------------------
// CompatibilityReport
// ---------------------------------------------------------------------------

#[test]
fn contract_compatibility_report() {
    let report = CompatibilityReport {
        assessments: vec![codeatlas_core::detector::CompatibilityAssessment {
            language: Language::Rust,
            status: SupportStatus::Supported,
            details: vec![CompatibilityDetail {
                feature: "Cargo workspace".to_string(),
                status: SupportStatus::Supported,
                explanation: "Fully supported".to_string(),
            }],
        }],
        is_provisional: true,
    };
    let json = serde_json::to_value(&report).unwrap();

    assert!(json.get("assessments").is_some(), "missing assessments");
    assert!(json.get("isProvisional").is_some(), "missing isProvisional");
    assert!(json.get("is_provisional").is_none(), "snake_case leak: is_provisional");

    assert_eq!(json["isProvisional"], true);

    // Verify nested assessment structure
    let assessment = &json["assessments"][0];
    assert!(assessment.get("language").is_some());
    assert!(assessment.get("status").is_some());
    assert!(assessment.get("details").is_some());
}

// ---------------------------------------------------------------------------
// GraphHealth
// ---------------------------------------------------------------------------

#[test]
fn contract_graph_health() {
    let health = GraphHealth {
        total_nodes: 42,
        resolved_edges: 30,
        unresolved_imports: 5,
        parse_failures: 2,
        unsupported_constructs: 3,
        unresolved_import_details: vec![UnresolvedImport {
            source_file: "src/main.rs".to_string(),
            specifier: "external_crate".to_string(),
            reason: UnresolvedReason::ExternalCrate,
        }],
    };
    let json = serde_json::to_value(&health).unwrap();

    assert!(json.get("totalNodes").is_some(), "missing totalNodes");
    assert!(json.get("resolvedEdges").is_some(), "missing resolvedEdges");
    assert!(json.get("unresolvedImports").is_some(), "missing unresolvedImports");
    assert!(json.get("parseFailures").is_some(), "missing parseFailures");
    assert!(json.get("unsupportedConstructs").is_some(), "missing unsupportedConstructs");
    assert!(json.get("unresolvedImportDetails").is_some(), "missing unresolvedImportDetails");

    // No snake_case leaks
    assert!(json.get("total_nodes").is_none(), "snake_case leak: total_nodes");
    assert!(json.get("resolved_edges").is_none(), "snake_case leak: resolved_edges");
    assert!(json.get("unresolved_imports").is_none(), "snake_case leak: unresolved_imports");
    assert!(json.get("parse_failures").is_none(), "snake_case leak: parse_failures");
    assert!(json.get("unsupported_constructs").is_none(), "snake_case leak: unsupported_constructs");
    assert!(json.get("unresolved_import_details").is_none(), "snake_case leak: unresolved_import_details");

    assert_eq!(json["totalNodes"], 42);
    assert_eq!(json["resolvedEdges"], 30);
}

// ---------------------------------------------------------------------------
// GraphProfile
// ---------------------------------------------------------------------------

#[test]
fn contract_graph_profile() {
    let profile = GraphProfile {
        languages: vec![Language::Rust, Language::TypeScript],
        package_manager: Some("cargo + pnpm".to_string()),
        resolution_mode: Some("bundler".to_string()),
        cargo_features: vec!["serde".to_string()],
        fingerprint: codeatlas_core::profile::ProfileFingerprint::empty(),
    };
    let json = serde_json::to_value(&profile).unwrap();

    assert!(json.get("languages").is_some(), "missing languages");
    assert!(json.get("packageManager").is_some(), "missing packageManager");
    assert!(json.get("resolutionMode").is_some(), "missing resolutionMode");
    assert!(json.get("cargoFeatures").is_some(), "missing cargoFeatures");
    assert!(json.get("fingerprint").is_some(), "missing fingerprint");

    // No snake_case leaks
    assert!(json.get("package_manager").is_none(), "snake_case leak: package_manager");
    assert!(json.get("resolution_mode").is_none(), "snake_case leak: resolution_mode");
    assert!(json.get("cargo_features").is_none(), "snake_case leak: cargo_features");

    assert_eq!(json["packageManager"], "cargo + pnpm");
    assert_eq!(json["resolutionMode"], "bundler");
}

// ---------------------------------------------------------------------------
// UnsupportedConstruct
// ---------------------------------------------------------------------------

#[test]
fn contract_unsupported_construct() {
    let construct = UnsupportedConstruct {
        construct_type: UnsupportedConstructType::CfgGate,
        location: SourceLocation {
            path: "src/lib.rs".into(),
            start_line: 10,
            end_line: 15,
        },
        impact: "Module may be conditionally compiled".to_string(),
        how_to_address: "Add cfg features to profile".to_string(),
    };
    let json = serde_json::to_value(&construct).unwrap();

    assert!(json.get("constructType").is_some(), "missing constructType");
    assert!(json.get("location").is_some(), "missing location");
    assert!(json.get("impact").is_some(), "missing impact");
    assert!(json.get("howToAddress").is_some(), "missing howToAddress");

    // No snake_case leaks
    assert!(json.get("construct_type").is_none(), "snake_case leak: construct_type");
    assert!(json.get("how_to_address").is_none(), "snake_case leak: how_to_address");

    assert_eq!(json["constructType"], "cfgGate");
}

// ---------------------------------------------------------------------------
// UnresolvedImport
// ---------------------------------------------------------------------------

#[test]
fn contract_unresolved_import() {
    let import = UnresolvedImport {
        source_file: "src/main.ts".to_string(),
        specifier: "./missing".to_string(),
        reason: UnresolvedReason::NoMatchingFile,
    };
    let json = serde_json::to_value(&import).unwrap();

    assert!(json.get("sourceFile").is_some(), "missing sourceFile");
    assert!(json.get("specifier").is_some(), "missing specifier");
    assert!(json.get("reason").is_some(), "missing reason");

    // No snake_case leaks
    assert!(json.get("source_file").is_none(), "snake_case leak: source_file");

    assert_eq!(json["sourceFile"], "src/main.ts");
    assert_eq!(json["specifier"], "./missing");
}

// ---------------------------------------------------------------------------
// UnresolvedReason (adjacently tagged enum)
// ---------------------------------------------------------------------------

#[test]
fn contract_unresolved_reason_unit_variants() {
    let cases: Vec<(UnresolvedReason, &str)> = vec![
        (UnresolvedReason::ExternalPackage, "externalPackage"),
        (UnresolvedReason::NoMatchingFile, "noMatchingFile"),
        (UnresolvedReason::DynamicImport, "dynamicImport"),
        (UnresolvedReason::CommonJsRequire, "commonJsRequire"),
        (UnresolvedReason::PathAliasNotMatched, "pathAliasNotMatched"),
        (UnresolvedReason::ExternalCrate, "externalCrate"),
        (UnresolvedReason::UnresolvablePath, "unresolvablePath"),
    ];

    for (variant, expected_type) in cases {
        let json = serde_json::to_value(&variant).unwrap();
        assert_eq!(
            json["type"], expected_type,
            "UnresolvedReason::{expected_type} should serialize with type={expected_type}"
        );
    }
}

#[test]
fn contract_unresolved_reason_other_variant() {
    let reason = UnresolvedReason::Other("custom reason".to_string());
    let json = serde_json::to_value(&reason).unwrap();

    assert_eq!(json["type"], "other");
    assert_eq!(json["data"], "custom reason");
}

// ---------------------------------------------------------------------------
// EdgeCategory
// ---------------------------------------------------------------------------

#[test]
fn contract_edge_category() {
    assert_eq!(serde_json::to_value(EdgeCategory::Value).unwrap(), "value");
    assert_eq!(serde_json::to_value(EdgeCategory::TypeOnly).unwrap(), "typeOnly");
    assert_eq!(serde_json::to_value(EdgeCategory::Dev).unwrap(), "dev");
    assert_eq!(serde_json::to_value(EdgeCategory::Build).unwrap(), "build");
    assert_eq!(serde_json::to_value(EdgeCategory::Test).unwrap(), "test");
    assert_eq!(serde_json::to_value(EdgeCategory::Peer).unwrap(), "peer");
    assert_eq!(serde_json::to_value(EdgeCategory::Normal).unwrap(), "normal");
    assert_eq!(serde_json::to_value(EdgeCategory::Manual).unwrap(), "manual");
}

// ---------------------------------------------------------------------------
// NodeKind
// ---------------------------------------------------------------------------

#[test]
fn contract_node_kind() {
    assert_eq!(serde_json::to_value(NodeKind::Package).unwrap(), "package");
    assert_eq!(serde_json::to_value(NodeKind::Module).unwrap(), "module");
    assert_eq!(serde_json::to_value(NodeKind::File).unwrap(), "file");
}

// ---------------------------------------------------------------------------
// EdgeKind
// ---------------------------------------------------------------------------

#[test]
fn contract_edge_kind() {
    assert_eq!(serde_json::to_value(EdgeKind::Imports).unwrap(), "imports");
    assert_eq!(serde_json::to_value(EdgeKind::ReExports).unwrap(), "reExports");
    assert_eq!(serde_json::to_value(EdgeKind::Contains).unwrap(), "contains");
    assert_eq!(serde_json::to_value(EdgeKind::DependsOn).unwrap(), "dependsOn");
    assert_eq!(serde_json::to_value(EdgeKind::Manual).unwrap(), "manual");
}

// ---------------------------------------------------------------------------
// OverlayStatus (adjacently tagged)
// ---------------------------------------------------------------------------

#[test]
fn contract_overlay_status_none() {
    let json = serde_json::to_value(OverlayStatus::None).unwrap();
    assert_eq!(json["type"], "none");
    // Unit variant: no "data" key
    assert!(json.get("data").is_none(), "none variant should not have data");
}

#[test]
fn contract_overlay_status_suppressed() {
    let suppressed = OverlayStatus::Suppressed {
        reason: "dead code".to_string(),
    };
    let json = serde_json::to_value(&suppressed).unwrap();
    assert_eq!(json["type"], "suppressed");
    assert_eq!(json["data"]["reason"], "dead code");
}

// ---------------------------------------------------------------------------
// ParseFailure
// ---------------------------------------------------------------------------

#[test]
fn contract_parse_failure() {
    let failure = ParseFailure {
        path: "src/broken.rs".into(),
        reason: "syntax error at line 42".to_string(),
    };
    let json = serde_json::to_value(&failure).unwrap();

    assert!(json.get("path").is_some(), "missing path");
    assert!(json.get("reason").is_some(), "missing reason");

    assert_eq!(json["path"], "src/broken.rs");
    assert_eq!(json["reason"], "syntax error at line 42");
}

// ---------------------------------------------------------------------------
// SourceLocation
// ---------------------------------------------------------------------------

#[test]
fn contract_source_location() {
    let loc = SourceLocation {
        path: "src/graph/types.rs".into(),
        start_line: 10,
        end_line: 20,
    };
    let json = serde_json::to_value(&loc).unwrap();

    assert!(json.get("path").is_some(), "missing path");
    assert!(json.get("startLine").is_some(), "missing startLine");
    assert!(json.get("endLine").is_some(), "missing endLine");

    // No snake_case leaks
    assert!(json.get("start_line").is_none(), "snake_case leak: start_line");
    assert!(json.get("end_line").is_none(), "snake_case leak: end_line");

    assert_eq!(json["path"], "src/graph/types.rs");
    assert_eq!(json["startLine"], 10);
    assert_eq!(json["endLine"], 20);
}
