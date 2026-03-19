import { describe, expect, it } from "vitest";
import type { EdgeCategory } from "@/types/graph";
import {
	type AppEdge,
	type AppNode,
	computeInitialExpanded,
	keyToId,
	type ProjectionInput,
	project,
} from "./graph-projection";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeNode(
	entityKind: "package" | "module" | "file",
	path: string,
	label: string,
	parentId?: string,
): AppNode {
	const key = { language: "typescript" as const, entityKind, relativePath: path };
	return {
		id: keyToId(key),
		type: entityKind,
		position: { x: 0, y: 0 },
		data: {
			label,
			kind: entityKind,
			language: "typescript",
			materializedKey: key,
			parentKey: null,
			isExpanded: false,
			childCount: 0,
			unsupportedConstructs: 0,
		},
		parentId,
	};
}

function makeEdge(sourceId: string, targetId: string, category: EdgeCategory = "value"): AppEdge {
	const id = `edge:${sourceId}→${targetId}:${category}`;
	return {
		id,
		source: sourceId,
		target: targetId,
		type: "dependency",
		data: {
			category,
			kind: "imports",
			isManual: false,
			isSuppressed: false,
			isBundled: false,
			bundledEdgeIds: [],
			bundledCount: 0,
			confidence: "syntactic",
			edgeId: id,
			sourceLocation: null,
			resolutionMethod: null,
			suppressionReason: null,
		},
	};
}

const ALL_CATEGORIES: Set<EdgeCategory> = new Set([
	"value",
	"typeOnly",
	"dev",
	"build",
	"test",
	"peer",
	"normal",
	"manual",
]);

function defaultInput(overrides: Partial<ProjectionInput> = {}): ProjectionInput {
	return {
		discoveredNodes: [],
		discoveredEdges: [],
		overlayEdges: [],
		suppressedEdgeIds: new Set(),
		expandedNodeIds: new Set(),
		categoryFilter: ALL_CATEGORIES,
		showSuppressed: false,
		...overrides,
	};
}

// ---------------------------------------------------------------------------
// Build a small test graph: 2 packages with modules + files
// ---------------------------------------------------------------------------

function buildTestGraph(): {
	nodes: AppNode[];
	edges: AppEdge[];
} {
	const pkgA = makeNode("package", "packages/a", "@pkg/a");
	const modA1 = makeNode("module", "packages/a/src", "src", pkgA.id);
	const fileA1 = makeNode("file", "packages/a/src/index.ts", "index.ts", modA1.id);
	const fileA2 = makeNode("file", "packages/a/src/utils.ts", "utils.ts", modA1.id);

	const pkgB = makeNode("package", "packages/b", "@pkg/b");
	const modB1 = makeNode("module", "packages/b/src", "src", pkgB.id);
	const fileB1 = makeNode("file", "packages/b/src/index.ts", "index.ts", modB1.id);
	const fileB2 = makeNode("file", "packages/b/src/helper.ts", "helper.ts", modB1.id);

	const nodes = [pkgA, modA1, fileA1, fileA2, pkgB, modB1, fileB1, fileB2];

	// fileA1 → fileB1 (value), fileA1 → fileB2 (typeOnly), fileA2 → fileB1 (dev)
	const edges = [
		makeEdge(fileA1.id, fileB1.id, "value"),
		makeEdge(fileA1.id, fileB2.id, "typeOnly"),
		makeEdge(fileA2.id, fileB1.id, "dev"),
	];

	return { nodes, edges };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("project()", () => {
	it("returns all nodes and edges when everything is expanded", () => {
		const { nodes, edges } = buildTestGraph();
		const allExpanded = new Set(nodes.filter((n) => n.type !== "file").map((n) => n.id));

		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: allExpanded,
			}),
		);

		expect(result.nodes).toHaveLength(nodes.length);
		expect(result.edges).toHaveLength(edges.length);
		expect(result.nodes.every((n) => !n.hidden)).toBe(true);
	});

	it("hides descendants of collapsed packages", () => {
		const { nodes, edges } = buildTestGraph();
		// Don't expand anything — all collapsed
		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: new Set(),
			}),
		);

		// Packages should be visible, modules and files hidden
		const visible = result.nodes.filter((n) => !n.hidden);
		expect(visible.every((n) => n.type === "package")).toBe(true);
		expect(visible).toHaveLength(2);
	});

	it("creates bundled edges between two collapsed packages", () => {
		const { nodes, edges } = buildTestGraph();
		// All collapsed
		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: new Set(),
			}),
		);

		const bundled = result.edges.filter((e) => e.data.isBundled);
		expect(bundled.length).toBeGreaterThanOrEqual(1);

		// The bundled edge from pkgA → pkgB should contain 3 underlying edges
		const pkgAId = keyToId({
			language: "typescript",
			entityKind: "package",
			relativePath: "packages/a",
		});
		const pkgBId = keyToId({
			language: "typescript",
			entityKind: "package",
			relativePath: "packages/b",
		});
		const aToBBundle = bundled.find((e) => e.source === pkgAId && e.target === pkgBId);
		expect(aToBBundle).toBeDefined();
		expect(aToBBundle?.data.bundledEdgeIds).toHaveLength(3);
		expect(aToBBundle?.data.bundledCount).toBe(3);
	});

	it("unbundles edges when a package is expanded", () => {
		const { nodes, edges } = buildTestGraph();
		const pkgAId = keyToId({
			language: "typescript",
			entityKind: "package",
			relativePath: "packages/a",
		});
		const modA1Id = keyToId({
			language: "typescript",
			entityKind: "module",
			relativePath: "packages/a/src",
		});

		// Expand pkgA and its module, keep pkgB collapsed
		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: new Set([pkgAId, modA1Id]),
			}),
		);

		// pkgA's children should be visible, pkgB's hidden
		const visibleIds = result.nodes.filter((n) => !n.hidden).map((n) => n.id);
		expect(visibleIds).toContain(pkgAId);
		expect(visibleIds).toContain(modA1Id);

		// Edges from fileA* to pkgB should be bundled (target is collapsed)
		const bundled = result.edges.filter((e) => e.data.isBundled);
		expect(bundled.length).toBeGreaterThanOrEqual(1);
	});

	it("filters edges by category", () => {
		const { nodes, edges } = buildTestGraph();
		const allExpanded = new Set(nodes.filter((n) => n.type !== "file").map((n) => n.id));

		// Only show value edges
		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: allExpanded,
				categoryFilter: new Set(["value"] as EdgeCategory[]),
			}),
		);

		expect(result.edges).toHaveLength(1);
		expect(result.edges[0].data.category).toBe("value");
	});

	it("category filtering affects bundled edge counts", () => {
		const { nodes, edges } = buildTestGraph();
		// All collapsed, only value category
		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: new Set(),
				categoryFilter: new Set(["value"] as EdgeCategory[]),
			}),
		);

		const bundled = result.edges.filter((e) => e.data.isBundled);
		if (bundled.length > 0) {
			// Only 1 value edge between the packages
			expect(bundled[0].data.bundledEdgeIds).toHaveLength(1);
		}
	});

	it("hides suppressed edges when showSuppressed is false", () => {
		const { nodes, edges } = buildTestGraph();
		const allExpanded = new Set(nodes.filter((n) => n.type !== "file").map((n) => n.id));
		const suppressedIds = new Set([edges[0].data.edgeId]);

		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: allExpanded,
				suppressedEdgeIds: suppressedIds,
				showSuppressed: false,
			}),
		);

		expect(result.edges).toHaveLength(2); // 3 - 1 suppressed
	});

	it("marks suppressed edges when showSuppressed is true", () => {
		const { nodes, edges } = buildTestGraph();
		const allExpanded = new Set(nodes.filter((n) => n.type !== "file").map((n) => n.id));
		const suppressedIds = new Set([edges[0].data.edgeId]);

		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: allExpanded,
				suppressedEdgeIds: suppressedIds,
				showSuppressed: true,
			}),
		);

		expect(result.edges).toHaveLength(3);
		const suppressed = result.edges.find((e) => e.data.isSuppressed);
		expect(suppressed).toBeDefined();
	});

	it("merges overlay edges with isManual flag", () => {
		const { nodes, edges } = buildTestGraph();
		const allExpanded = new Set(nodes.filter((n) => n.type !== "file").map((n) => n.id));

		const overlayEdge: AppEdge = {
			id: "manual:1",
			source: nodes[2].id, // fileA1
			target: nodes[6].id, // fileB1
			type: "dependency",
			data: {
				category: "manual",
				kind: "manual",
				isManual: true,
				isSuppressed: false,
				isBundled: false,
				bundledEdgeIds: [],
				bundledCount: 0,
				confidence: "structural",
				edgeId: "manual:1",
				sourceLocation: null,
				resolutionMethod: null,
				suppressionReason: null,
			},
		};

		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: allExpanded,
				overlayEdges: [overlayEdge],
			}),
		);

		expect(result.edges).toHaveLength(4); // 3 discovered + 1 manual
		const manual = result.edges.find((e) => e.data.isManual);
		expect(manual).toBeDefined();
		expect(manual?.data.category).toBe("manual");
	});

	it("parents appear before children in output", () => {
		const { nodes, edges } = buildTestGraph();
		const allExpanded = new Set(nodes.filter((n) => n.type !== "file").map((n) => n.id));

		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: allExpanded,
			}),
		);

		// For each node with a parentId, its parent must appear earlier
		const indexMap = new Map(result.nodes.map((n, i) => [n.id, i]));
		for (const node of result.nodes) {
			if (node.parentId) {
				const parentIdx = indexMap.get(node.parentId);
				const childIdx = indexMap.get(node.id);
				expect(parentIdx).toBeDefined();
				expect(childIdx).toBeDefined();
				if (parentIdx !== undefined && childIdx !== undefined) {
					expect(parentIdx).toBeLessThan(childIdx);
				}
			}
		}
	});
});

describe("computeInitialExpanded()", () => {
	it("expands all packages and modules for small graphs (<120)", () => {
		const { nodes } = buildTestGraph(); // 8 nodes
		const expanded = computeInitialExpanded(nodes);

		const compoundIds = nodes
			.filter((n) => n.type === "package" || n.type === "module")
			.map((n) => n.id);

		for (const id of compoundIds) {
			expect(expanded.has(id)).toBe(true);
		}
	});

	it("expands only root packages for medium graphs (120-250)", () => {
		// Create 130 nodes: 2 packages, 4 modules, 124 files
		const pkg1 = makeNode("package", "pkg/a", "@pkg/a");
		const pkg2 = makeNode("package", "pkg/b", "@pkg/b");
		const mod1 = makeNode("module", "pkg/a/src", "src", pkg1.id);
		const mod2 = makeNode("module", "pkg/b/src", "src", pkg2.id);

		const files: AppNode[] = Array.from({ length: 126 }, (_, i) =>
			makeNode("file", `pkg/a/src/file-${i}.ts`, `file-${i}.ts`, mod1.id),
		);

		const nodes = [pkg1, pkg2, mod1, mod2, ...files]; // 130 total
		const expanded = computeInitialExpanded(nodes);

		// Root packages should be expanded
		expect(expanded.has(pkg1.id)).toBe(true);
		expect(expanded.has(pkg2.id)).toBe(true);
		// Modules should NOT be expanded (medium range = package-level only)
		expect(expanded.has(mod1.id)).toBe(false);
		expect(expanded.has(mod2.id)).toBe(false);
	});

	it("returns empty set for large graphs (>250)", () => {
		// Create 251 fake nodes
		const nodes: AppNode[] = Array.from({ length: 251 }, (_, i) =>
			makeNode("file", `file-${i}.ts`, `file-${i}.ts`),
		);
		const expanded = computeInitialExpanded(nodes);
		expect(expanded.size).toBe(0);
	});
});

describe("keyToId()", () => {
	it("produces deterministic string ID from MaterializedKey", () => {
		const key = {
			language: "typescript" as const,
			entityKind: "file" as const,
			relativePath: "src/index.ts",
		};
		expect(keyToId(key)).toBe("typescript:file:src/index.ts");
	});
});

// M6: Overlay projection tests
describe("overlay projection", () => {
	it("suppressed edges are hidden when showSuppressed is false", () => {
		const { nodes, edges } = buildTestGraph();
		const allExpanded = new Set(nodes.filter((n) => n.type !== "file").map((n) => n.id));

		// Suppress the first edge
		const suppressedId = edges[0].data.edgeId;
		const suppressedIds = new Set([suppressedId]);

		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: allExpanded,
				suppressedEdgeIds: suppressedIds,
				showSuppressed: false,
			}),
		);

		// Suppressed edge should be hidden
		const suppressed = result.edges.find((e) => e.data.edgeId === suppressedId);
		expect(suppressed).toBeUndefined();
	});

	it("suppressed edges are visible when showSuppressed is true", () => {
		const { nodes, edges } = buildTestGraph();
		const allExpanded = new Set(nodes.filter((n) => n.type !== "file").map((n) => n.id));

		const suppressedId = edges[0].data.edgeId;
		const suppressedIds = new Set([suppressedId]);

		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				expandedNodeIds: allExpanded,
				suppressedEdgeIds: suppressedIds,
				showSuppressed: true,
			}),
		);

		// Suppressed edge should be present and marked
		const suppressed = result.edges.find((e) => e.data.edgeId === suppressedId);
		expect(suppressed).toBeDefined();
		expect(suppressed?.data.isSuppressed).toBe(true);
	});

	it("manual edges appear in projected output with isManual flag", () => {
		const { nodes, edges } = buildTestGraph();
		const allExpanded = new Set(nodes.filter((n) => n.type !== "file").map((n) => n.id));

		const manualEdge: AppEdge = {
			id: "manual:test",
			source: nodes[2].id,
			target: nodes[6].id,
			type: "dependency",
			data: {
				category: "manual",
				kind: "manual",
				isManual: true,
				isSuppressed: false,
				isBundled: false,
				bundledEdgeIds: [],
				bundledCount: 0,
				confidence: "structural",
				edgeId: "manual:test",
				sourceLocation: null,
				resolutionMethod: "manual config",
				suppressionReason: null,
			},
		};

		const result = project(
			defaultInput({
				discoveredNodes: nodes,
				discoveredEdges: edges,
				overlayEdges: [manualEdge],
				expandedNodeIds: allExpanded,
			}),
		);

		const manual = result.edges.find((e) => e.data.edgeId === "manual:test");
		expect(manual).toBeDefined();
		expect(manual?.data.isManual).toBe(true);
		expect(manual?.data.category).toBe("manual");
	});
});
