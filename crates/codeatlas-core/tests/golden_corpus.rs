//! Golden corpus integration tests for Code Atlas M8.
//!
//! Each test scans a fixture (or the project itself) through the full
//! `run_scan` pipeline and asserts on the shape and content of the output.

use std::collections::HashSet;
use std::sync::Mutex;

use camino::Utf8Path;
use tokio_util::sync::CancellationToken;

use codeatlas_core::config::RepoConfig;
use codeatlas_core::detector::{Detector, RustDetector, TypeScriptDetector};
use codeatlas_core::graph::types::{
    EdgeCategory, EdgeKind, Language, NodeKind, ParseFailure, UnresolvedImport,
    UnsupportedConstruct, UnsupportedConstructType,
};
use codeatlas_core::graph::identity::MaterializedKey;
use codeatlas_core::health::compatibility::CompatibilityReport;
use codeatlas_core::health::graph_health::GraphHealth;
use codeatlas_core::scan::{ScanPhase, ScanSink};
use codeatlas_core::{run_scan, GraphProfile};

// ---------------------------------------------------------------------------
// CollectingScanSink — captures all streamed output for assertion
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CollectingScanSink {
    phases: Mutex<Vec<(ScanPhase, Vec<codeatlas_core::graph::types::NodeData>, Vec<codeatlas_core::graph::types::EdgeData>)>>,
    health: Mutex<Option<GraphHealth>>,
    compatibility: Mutex<Option<CompatibilityReport>>,
    details: Mutex<Option<(Vec<UnsupportedConstruct>, Vec<ParseFailure>, Vec<UnresolvedImport>)>>,
    overlay: Mutex<Option<(Vec<codeatlas_core::graph::types::EdgeData>, Vec<String>)>>,
}

impl CollectingScanSink {
    fn health(&self) -> Option<GraphHealth> {
        self.health.lock().expect("lock").clone()
    }
    fn compatibility(&self) -> Option<CompatibilityReport> {
        self.compatibility.lock().expect("lock").clone()
    }
    fn phases(&self) -> Vec<(ScanPhase, Vec<codeatlas_core::graph::types::NodeData>, Vec<codeatlas_core::graph::types::EdgeData>)> {
        self.phases.lock().expect("lock").clone()
    }
    #[allow(dead_code)]
    fn details(&self) -> Option<(Vec<UnsupportedConstruct>, Vec<ParseFailure>, Vec<UnresolvedImport>)> {
        self.details.lock().expect("lock").clone()
    }
}

impl ScanSink for CollectingScanSink {
    fn on_compatibility(&self, report: CompatibilityReport) {
        *self.compatibility.lock().expect("lock") = Some(report);
    }
    fn on_phase(
        &self,
        phase: ScanPhase,
        nodes: Vec<codeatlas_core::graph::types::NodeData>,
        edges: Vec<codeatlas_core::graph::types::EdgeData>,
    ) {
        self.phases.lock().expect("lock").push((phase, nodes, edges));
    }
    fn on_health(&self, health: GraphHealth) {
        *self.health.lock().expect("lock") = Some(health);
    }
    fn on_progress(&self, _scanned: usize, _total: usize) {}
    fn on_details(
        &self,
        unsupported_constructs: Vec<UnsupportedConstruct>,
        parse_failures: Vec<ParseFailure>,
        unresolved_imports: Vec<UnresolvedImport>,
    ) {
        *self.details.lock().expect("lock") =
            Some((unsupported_constructs, parse_failures, unresolved_imports));
    }
    fn on_overlay(
        &self,
        manual_edges: Vec<codeatlas_core::graph::types::EdgeData>,
        suppressed_edge_ids: Vec<String>,
    ) {
        *self.overlay.lock().expect("lock") = Some((manual_edges, suppressed_edge_ids));
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn project_root() -> &'static Utf8Path {
    Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("should find project root (two levels above codeatlas-core)")
}

fn fixture_path(relative: &str) -> camino::Utf8PathBuf {
    Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

// ---------------------------------------------------------------------------
// 1. Self-scan — scan the project's own workspace root
// ---------------------------------------------------------------------------

#[test]
fn golden_corpus_self_scan() {
    let root = project_root();
    let workspace =
        codeatlas_core::workspace::discover_workspace(root).expect("workspace discovery");
    let config = RepoConfig::load_from_dir(&workspace.root)
        .unwrap_or_else(|_| RepoConfig::default_config());
    let profile = GraphProfile::detect_from_workspace(&workspace);
    let detectors: Vec<Box<dyn Detector>> =
        vec![Box::new(RustDetector), Box::new(TypeScriptDetector)];
    let cancel = CancellationToken::new();
    let sink = CollectingScanSink::default();

    let results =
        run_scan(&workspace, &profile, &config, &detectors, &sink, &cancel)
            .expect("self-scan should succeed");

    // --- packages ---
    let pkg_nodes: Vec<_> = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Package)
        .collect();
    assert!(
        pkg_nodes.len() >= 2,
        "should find at least 2 Rust packages (codeatlas-core, codeatlas-tauri), got {}",
        pkg_nodes.len()
    );

    // --- language ---
    let has_rust_pkg = pkg_nodes.iter().any(|n| n.language == Language::Rust);
    assert!(has_rust_pkg, "should have Rust packages");

    // --- compatibility ---
    let compat = sink
        .compatibility()
        .expect("compatibility report should be streamed");
    assert!(!compat.is_provisional, "post-scan report must not be provisional");
    assert!(
        compat
            .assessments
            .iter()
            .any(|a| a.language == Language::Rust),
        "compatibility should include a Rust assessment"
    );

    // --- modules/files ---
    let file_count = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::File)
        .count();
    assert!(
        file_count > 10,
        "should discover >10 files in this project, got {file_count}"
    );

    // --- import edges ---
    let import_edges = results
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .count();
    assert!(import_edges > 0, "should have import edges");

    // --- no duplicate MaterializedKeys ---
    let all_keys: Vec<&MaterializedKey> =
        results.nodes.iter().map(|n| &n.materialized_key).collect();
    let unique_keys: HashSet<&MaterializedKey> = all_keys.iter().copied().collect();
    assert_eq!(
        all_keys.len(),
        unique_keys.len(),
        "no duplicate MaterializedKeys"
    );

    // --- health ---
    let health = sink.health().expect("health should be streamed");
    assert!(health.total_nodes > 0);
    assert!(health.resolved_edges > 0);
    // The project's own scan has external crate imports (serde, etc.) that
    // cannot be resolved, so unresolved_imports must be > 0.
    assert!(
        health.unresolved_imports > 0,
        "self-scan must have unresolved imports (external crates), got {}",
        health.unresolved_imports
    );

    // --- 3 phases streamed ---
    let phases = sink.phases();
    assert_eq!(
        phases.len(),
        3,
        "exactly 3 phases expected (PackageTopology, ModuleStructure, FileEdges), got {}",
        phases.len()
    );
    assert_eq!(phases[0].0, ScanPhase::PackageTopology);
    assert_eq!(phases[1].0, ScanPhase::ModuleStructure);
    assert_eq!(phases[2].0, ScanPhase::FileEdges);
}

// ---------------------------------------------------------------------------
// 2. rust-workspace fixture
// ---------------------------------------------------------------------------

#[test]
fn golden_corpus_rust_workspace() {
    let fixture_dir = fixture_path("../../tests/fixtures/rust-workspace");
    if !fixture_dir.exists() {
        eprintln!("skipping: fixture not found at {fixture_dir}");
        return;
    }

    let workspace =
        codeatlas_core::workspace::discover_workspace(&fixture_dir).expect("discovery");
    let config = RepoConfig::default_config();
    let profile = GraphProfile::detect_from_workspace(&workspace);
    let detectors: Vec<Box<dyn Detector>> = vec![Box::new(RustDetector)];
    let cancel = CancellationToken::new();
    let sink = CollectingScanSink::default();

    let results =
        run_scan(&workspace, &profile, &config, &detectors, &sink, &cancel)
            .expect("scan should succeed");

    // --- exactly 2 packages ---
    let pkg_nodes: Vec<_> = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Package)
        .collect();
    assert_eq!(
        pkg_nodes.len(),
        2,
        "should find exactly 2 packages (fixture-cli, fixture-core), got {}",
        pkg_nodes.len()
    );
    let pkg_names: HashSet<&str> = pkg_nodes.iter().map(|n| n.label.as_str()).collect();
    assert!(pkg_names.contains("fixture-cli"), "should find fixture-cli");
    assert!(pkg_names.contains("fixture-core"), "should find fixture-core");

    // --- inter-crate dependency edge ---
    let dep_edges: Vec<_> = results
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::DependsOn)
        .collect();
    assert!(
        !dep_edges.is_empty(),
        "should have inter-crate dependency edges"
    );
    // fixture-cli depends on fixture-core
    let cli_to_core = dep_edges.iter().find(|e| {
        let source_is_cli = pkg_nodes
            .iter()
            .any(|n| n.label == "fixture-cli" && n.materialized_key == e.source_key);
        let target_is_core = pkg_nodes
            .iter()
            .any(|n| n.label == "fixture-core" && n.materialized_key == e.target_key);
        source_is_cli && target_is_core
    });
    assert!(
        cli_to_core.is_some(),
        "should find fixture-cli -> fixture-core dependency edge"
    );
    assert_eq!(
        cli_to_core.unwrap().category,
        EdgeCategory::Normal,
        "fixture-cli -> fixture-core dependency should be Normal category"
    );

    // --- modules/files from tree-sitter ---
    let mod_or_file = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Module || n.kind == NodeKind::File)
        .count();
    assert!(mod_or_file > 0, "should discover modules or files via tree-sitter");

    // --- build.rs detected as BuildScript unsupported construct ---
    let build_scripts: Vec<_> = results
        .unsupported_constructs
        .iter()
        .filter(|c| c.construct_type == UnsupportedConstructType::BuildScript)
        .collect();
    assert!(
        !build_scripts.is_empty(),
        "should detect build.rs as BuildScript unsupported construct"
    );

    // --- compatibility report shows Rust ---
    let compat = sink
        .compatibility()
        .expect("compatibility report");
    assert!(
        compat
            .assessments
            .iter()
            .any(|a| a.language == Language::Rust),
        "compatibility should include Rust assessment"
    );

    // --- unresolved imports ---
    // The fixture source files have no `use` statements for external crates
    // (serde is in Cargo.toml but not used in source), so unresolved imports
    // come only from actual `use` declarations the tree-sitter parser finds.
    // main.rs uses `fixture_core::hello()` as a path expression (not a use
    // declaration), so it is correctly resolved as a cross-crate import edge.
}

// ---------------------------------------------------------------------------
// 3. rust-unsupported fixture
// ---------------------------------------------------------------------------

#[test]
fn golden_corpus_rust_unsupported() {
    let fixture_dir = fixture_path("../../tests/fixtures/rust-unsupported");
    if !fixture_dir.exists() {
        eprintln!("skipping: fixture not found at {fixture_dir}");
        return;
    }

    let workspace =
        codeatlas_core::workspace::discover_workspace(&fixture_dir).expect("discovery");
    let config = RepoConfig::default_config();
    let profile = GraphProfile::detect_from_workspace(&workspace);
    let detectors: Vec<Box<dyn Detector>> = vec![Box::new(RustDetector)];
    let cancel = CancellationToken::new();
    let sink = CollectingScanSink::default();

    let results =
        run_scan(&workspace, &profile, &config, &detectors, &sink, &cancel)
            .expect("scan should succeed");

    // --- 1 package ---
    let pkg_nodes: Vec<_> = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Package)
        .collect();
    assert_eq!(
        pkg_nodes.len(),
        1,
        "should find exactly 1 package (fixture-unsupported), got {}",
        pkg_nodes.len()
    );
    assert_eq!(pkg_nodes[0].label, "fixture-unsupported");

    // --- BuildScript unsupported construct ---
    let build_scripts: Vec<_> = results
        .unsupported_constructs
        .iter()
        .filter(|c| c.construct_type == UnsupportedConstructType::BuildScript)
        .collect();
    assert!(
        !build_scripts.is_empty(),
        "should detect build.rs as BuildScript"
    );

    // --- CfgGate unsupported construct ---
    // lib.rs has `#[cfg(feature = "serde-support")] mod serde_impl;`
    let cfg_gates: Vec<_> = results
        .unsupported_constructs
        .iter()
        .filter(|c| c.construct_type == UnsupportedConstructType::CfgGate)
        .collect();
    assert!(
        !cfg_gates.is_empty(),
        "should detect #[cfg(...)] on mod as CfgGate"
    );

    // --- compatibility report ---
    let compat = sink
        .compatibility()
        .expect("compatibility report");
    assert!(
        compat
            .assessments
            .iter()
            .any(|a| a.language == Language::Rust),
        "compatibility should include Rust assessment"
    );
    assert!(
        !compat.is_provisional,
        "post-scan report must not be provisional"
    );
}

// ---------------------------------------------------------------------------
// 4. ts-monorepo fixture
// ---------------------------------------------------------------------------

#[test]
fn golden_corpus_ts_monorepo() {
    let fixture_dir = fixture_path("../../tests/fixtures/ts-monorepo");
    if !fixture_dir.exists() {
        eprintln!("skipping: fixture not found at {fixture_dir}");
        return;
    }

    let workspace =
        codeatlas_core::workspace::discover_workspace(&fixture_dir).expect("discovery");
    let config = RepoConfig::default_config();
    let profile = GraphProfile::detect_from_workspace(&workspace);
    // Only TypeScriptDetector
    let detectors: Vec<Box<dyn Detector>> = vec![Box::new(TypeScriptDetector)];
    let cancel = CancellationToken::new();
    let sink = CollectingScanSink::default();

    let results =
        run_scan(&workspace, &profile, &config, &detectors, &sink, &cancel)
            .expect("scan should succeed");

    // --- 2 packages ---
    let pkg_nodes: Vec<_> = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Package)
        .collect();
    assert_eq!(
        pkg_nodes.len(),
        2,
        "should find exactly 2 packages (@fixture/app, @fixture/shared), got {}",
        pkg_nodes.len()
    );
    let pkg_names: HashSet<&str> = pkg_nodes.iter().map(|n| n.label.as_str()).collect();
    assert!(pkg_names.contains("@fixture/app"), "should find @fixture/app");
    assert!(
        pkg_names.contains("@fixture/shared"),
        "should find @fixture/shared"
    );

    // --- inter-package dependency edge ---
    let dep_edges: Vec<_> = results
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::DependsOn)
        .collect();
    assert!(
        !dep_edges.is_empty(),
        "should have inter-package dependency edges"
    );
    // @fixture/app depends on @fixture/shared
    let app_to_shared = dep_edges.iter().find(|e| {
        let source_is_app = pkg_nodes
            .iter()
            .any(|n| n.label == "@fixture/app" && n.materialized_key == e.source_key);
        let target_is_shared = pkg_nodes
            .iter()
            .any(|n| n.label == "@fixture/shared" && n.materialized_key == e.target_key);
        source_is_app && target_is_shared
    });
    assert!(
        app_to_shared.is_some(),
        "should find @fixture/app -> @fixture/shared dependency edge"
    );

    // --- import edges with correct categories ---
    let import_edges: Vec<_> = results
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();
    assert!(!import_edges.is_empty(), "should have import edges");

    // Value imports exist (e.g., `import { greet } from "@fixture/shared"`)
    let value_imports: Vec<_> = import_edges
        .iter()
        .filter(|e| e.category == EdgeCategory::Value)
        .collect();
    assert!(
        !value_imports.is_empty(),
        "should have value import edges"
    );

    // Type-only imports exist (e.g., `import type { Config } from "@fixture/shared"`)
    let type_only_imports: Vec<_> = import_edges
        .iter()
        .filter(|e| e.category == EdgeCategory::TypeOnly)
        .collect();
    assert!(
        !type_only_imports.is_empty(),
        "should have type-only import edges"
    );

    // --- dynamic import detected as unsupported construct ---
    let dynamic_imports: Vec<_> = results
        .unsupported_constructs
        .iter()
        .filter(|c| c.construct_type == UnsupportedConstructType::DynamicImport)
        .collect();
    assert!(
        !dynamic_imports.is_empty(),
        "should detect dynamic import() as unsupported construct"
    );

    // --- unresolved imports (dynamic imports are also unresolved) ---
    assert!(
        !results.unresolved_imports.is_empty(),
        "should have unresolved imports (dynamic imports produce unresolved entries)"
    );
}

// ---------------------------------------------------------------------------
// 5. ts-unsupported fixture
// ---------------------------------------------------------------------------

#[test]
fn golden_corpus_ts_unsupported() {
    let fixture_dir = fixture_path("../../tests/fixtures/ts-unsupported");
    if !fixture_dir.exists() {
        eprintln!("skipping: fixture not found at {fixture_dir}");
        return;
    }

    let workspace =
        codeatlas_core::workspace::discover_workspace(&fixture_dir).expect("discovery");
    let config = RepoConfig::default_config();
    let profile = GraphProfile::detect_from_workspace(&workspace);
    // Only TypeScriptDetector
    let detectors: Vec<Box<dyn Detector>> = vec![Box::new(TypeScriptDetector)];
    let cancel = CancellationToken::new();
    let sink = CollectingScanSink::default();

    let results =
        run_scan(&workspace, &profile, &config, &detectors, &sink, &cancel)
            .expect("scan should succeed");

    // --- packages discovered ---
    let pkg_nodes: Vec<_> = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Package)
        .collect();
    assert!(
        !pkg_nodes.is_empty(),
        "should discover packages in ts-unsupported fixture"
    );

    // --- CommonJsRequire unsupported construct ---
    let require_constructs: Vec<_> = results
        .unsupported_constructs
        .iter()
        .filter(|c| c.construct_type == UnsupportedConstructType::CommonJsRequire)
        .collect();
    assert!(
        !require_constructs.is_empty(),
        "should detect require() as CommonJsRequire unsupported construct"
    );

    // --- DynamicImport unsupported construct ---
    let dynamic_constructs: Vec<_> = results
        .unsupported_constructs
        .iter()
        .filter(|c| c.construct_type == UnsupportedConstructType::DynamicImport)
        .collect();
    assert!(
        !dynamic_constructs.is_empty(),
        "should detect dynamic import() as DynamicImport unsupported construct"
    );

    // --- ExportsCondition unsupported construct ---
    // The @unsupported/lib package.json has a conditional exports field
    let exports_constructs: Vec<_> = results
        .unsupported_constructs
        .iter()
        .filter(|c| c.construct_type == UnsupportedConstructType::ExportsCondition)
        .collect();
    assert!(
        !exports_constructs.is_empty(),
        "should detect package.json exports conditions as ExportsCondition"
    );

    // --- unresolved imports include require() and dynamic imports ---
    assert!(
        !results.unresolved_imports.is_empty(),
        "should have unresolved imports"
    );
    let has_require_unresolved = results
        .unresolved_imports
        .iter()
        .any(|u| matches!(u.reason, codeatlas_core::graph::types::UnresolvedReason::CommonJsRequire));
    assert!(
        has_require_unresolved,
        "unresolved imports should include a CommonJsRequire entry"
    );
    let has_dynamic_unresolved = results
        .unresolved_imports
        .iter()
        .any(|u| matches!(u.reason, codeatlas_core::graph::types::UnresolvedReason::DynamicImport));
    assert!(
        has_dynamic_unresolved,
        "unresolved imports should include a DynamicImport entry"
    );
}
