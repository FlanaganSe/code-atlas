import { beforeEach, describe, expect, it } from "vitest";
import type { AppEdge, AppNode } from "@/store/graph-projection";
import { keyToId } from "@/store/graph-projection";
import type { EdgeCategory } from "@/types/graph";
import { useGraphStore } from "./graph-store";

// ---------------------------------------------------------------------------
// Helpers
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

function buildSmallGraph(): { nodes: AppNode[]; edges: AppEdge[] } {
	const pkg = makeNode("package", "pkg/a", "@pkg/a");
	const mod = makeNode("module", "pkg/a/src", "src", pkg.id);
	const file1 = makeNode("file", "pkg/a/src/index.ts", "index.ts", mod.id);
	const file2 = makeNode("file", "pkg/a/src/utils.ts", "utils.ts", mod.id);

	const nodes = [pkg, mod, file1, file2];
	const edges = [makeEdge(file1.id, file2.id, "value")];
	return { nodes, edges };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useGraphStore", () => {
	beforeEach(() => {
		// Reset store between tests
		useGraphStore.setState({
			discoveredNodes: [],
			discoveredEdges: [],
			overlayEdges: [],
			suppressedEdgeIds: new Set(),
			expandedNodeIds: new Set(),
			projectedNodes: [],
			projectedEdges: [],
			showSuppressed: false,
		});
	});

	it("loadFixture populates source data and projects", () => {
		const { nodes, edges } = buildSmallGraph();
		useGraphStore.getState().loadFixture(nodes, edges);

		const state = useGraphStore.getState();
		expect(state.discoveredNodes).toHaveLength(4);
		expect(state.discoveredEdges).toHaveLength(1);
		expect(state.projectedNodes.length).toBeGreaterThan(0);
	});

	it("loadFixture sets initial expanded state via adaptation rules", () => {
		const { nodes, edges } = buildSmallGraph();
		useGraphStore.getState().loadFixture(nodes, edges);

		const state = useGraphStore.getState();
		// Small graph (<120) — all packages and modules expanded
		const pkgId = nodes.find((n) => n.type === "package")?.id ?? "";
		const modId = nodes.find((n) => n.type === "module")?.id ?? "";
		expect(state.expandedNodeIds.has(pkgId)).toBe(true);
		expect(state.expandedNodeIds.has(modId)).toBe(true);
	});

	it("toggleExpand re-runs projection", () => {
		const { nodes, edges } = buildSmallGraph();
		useGraphStore.getState().loadFixture(nodes, edges);

		const beforeEdges = useGraphStore.getState().projectedEdges;

		// Collapse the package
		const pkg = nodes.find((n) => n.type === "package");
		expect(pkg).toBeDefined();
		if (!pkg) return;
		useGraphStore.getState().toggleExpand(pkg.id);

		const afterState = useGraphStore.getState();
		expect(afterState.expandedNodeIds.has(pkg.id)).toBe(false);
		// Edge should be hidden (both ends are inside collapsed package)
		const visibleEdges = afterState.projectedEdges.filter((e) => !e.hidden);
		expect(visibleEdges.length).toBeLessThanOrEqual(beforeEdges.length);
	});

	it("setCategoryFilter re-runs projection", () => {
		const { nodes, edges } = buildSmallGraph();
		useGraphStore.getState().loadFixture(nodes, edges);

		// Filter to only "dev" — should remove the "value" edge
		useGraphStore.getState().setCategoryFilter(new Set(["dev"] as EdgeCategory[]));

		const state = useGraphStore.getState();
		expect(state.projectedEdges).toHaveLength(0);
	});

	it("expandAll expands all compound nodes", () => {
		const { nodes, edges } = buildSmallGraph();
		useGraphStore.getState().loadFixture(nodes, edges);
		useGraphStore.getState().collapseAll();
		useGraphStore.getState().expandAll();

		const state = useGraphStore.getState();
		const compoundIds = nodes
			.filter((n) => n.type === "package" || n.type === "module")
			.map((n) => n.id);

		for (const id of compoundIds) {
			expect(state.expandedNodeIds.has(id)).toBe(true);
		}
	});

	it("collapseAll empties expanded set", () => {
		const { nodes, edges } = buildSmallGraph();
		useGraphStore.getState().loadFixture(nodes, edges);
		useGraphStore.getState().collapseAll();

		expect(useGraphStore.getState().expandedNodeIds.size).toBe(0);
	});

	it("toggleSuppressed re-runs projection", () => {
		const { nodes, edges } = buildSmallGraph();
		const suppressedIds = new Set([edges[0].data.edgeId]);
		useGraphStore.getState().loadFixture(nodes, edges, [], suppressedIds);

		// By default suppressed edges are hidden
		const before = useGraphStore.getState().projectedEdges.length;

		useGraphStore.getState().toggleSuppressed();

		const after = useGraphStore.getState().projectedEdges.length;
		expect(after).toBeGreaterThanOrEqual(before);
	});

	it("applyScanPhase adds nodes and edges from raw data", () => {
		const rawNodes = [
			{
				materializedKey: {
					language: "rust" as const,
					entityKind: "package" as const,
					relativePath: "crates/core",
				},
				lineageKey: null,
				label: "codeatlas-core",
				kind: "package" as const,
				language: "rust" as const,
				parentKey: null,
			},
		];
		const rawEdges: import("@/types/graph").EdgeData[] = [];

		useGraphStore.getState().applyScanPhase("packageTopology", rawNodes, rawEdges);

		const state = useGraphStore.getState();
		expect(state.discoveredNodes.length).toBe(1);
		expect(state.discoveredNodes[0].id).toBe("rust:package:crates/core");
		expect(state.discoveredNodes[0].data.label).toBe("codeatlas-core");
		expect(state.layoutVersion).toBeGreaterThan(0);
	});

	it("applyScanPhase deduplicates nodes on subsequent phases", () => {
		const pkg = {
			materializedKey: {
				language: "rust" as const,
				entityKind: "package" as const,
				relativePath: "crates/core",
			},
			lineageKey: null,
			label: "codeatlas-core",
			kind: "package" as const,
			language: "rust" as const,
			parentKey: null,
		};
		useGraphStore.getState().applyScanPhase("packageTopology", [pkg], []);

		const mod = {
			materializedKey: {
				language: "rust" as const,
				entityKind: "module" as const,
				relativePath: "crates/core/src/graph",
			},
			lineageKey: null,
			label: "graph",
			kind: "module" as const,
			language: "rust" as const,
			parentKey: pkg.materializedKey,
		};
		useGraphStore.getState().applyScanPhase("moduleStructure", [mod], []);

		const state = useGraphStore.getState();
		expect(state.discoveredNodes.length).toBe(2);
	});

	it("clearGraph resets all graph state", () => {
		const { nodes, edges } = buildSmallGraph();
		useGraphStore.getState().loadFixture(nodes, edges);

		useGraphStore.getState().clearGraph();

		const state = useGraphStore.getState();
		expect(state.discoveredNodes.length).toBe(0);
		expect(state.discoveredEdges.length).toBe(0);
		expect(state.projectedNodes.length).toBe(0);
		expect(state.projectedEdges.length).toBe(0);
	});
});
