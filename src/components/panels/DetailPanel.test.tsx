import { beforeEach, describe, expect, it } from "vitest";
import type { AppEdge, AppNode } from "@/store/graph-projection";
import { useGraphStore } from "@/store/graph-store";
import { useScanStore } from "@/store/scan-store";
import type { ParseFailure, UnsupportedConstruct } from "@/types/graph";

/** Create a minimal AppNode for testing. */
function makeNode(
	id: string,
	type: "package" | "module" | "file",
	label: string,
	parentId?: string,
): AppNode {
	const parts = id.split(":");
	return {
		id,
		type,
		position: { x: 0, y: 0 },
		data: {
			label,
			kind: type,
			language: parts[0] ?? "rust",
			materializedKey: {
				language: (parts[0] ?? "rust") as "rust" | "typescript",
				entityKind: type,
				relativePath: parts[2] ?? id,
			},
			parentKey: null,
			isExpanded: false,
			childCount: 0,
			unsupportedConstructs: 0,
		},
		parentId,
	};
}

/** Create a minimal AppEdge for testing. */
function makeEdge(
	id: string,
	source: string,
	target: string,
	category: import("@/types/graph").EdgeCategory = "value",
): AppEdge {
	return {
		id,
		source,
		target,
		type: "dependency",
		data: {
			category,
			kind: "imports",
			isManual: false,
			isSuppressed: false,
			isBundled: false,
			bundledEdgeIds: [id],
			bundledCount: 1,
			confidence: "syntactic",
			edgeId: id,
			sourceLocation: null,
			resolutionMethod: null,
			suppressionReason: null,
		},
	};
}

describe("DetailPanel store integration", () => {
	beforeEach(() => {
		useGraphStore.setState({
			discoveredNodes: [],
			discoveredEdges: [],
			overlayEdges: [],
			suppressedEdgeIds: new Set(),
			expandedNodeIds: new Set(),
			selectedNodeId: null,
			projectedNodes: [],
			projectedEdges: [],
			pendingViewport: null,
		});
		useScanStore.setState({
			unsupportedConstructs: [],
			parseFailures: [],
		});
	});

	it("selectNode sets selectedNodeId", () => {
		const node = makeNode("rust:package:crate-a", "package", "crate-a");
		useGraphStore.setState({ discoveredNodes: [node] });
		useGraphStore.getState().selectNode("rust:package:crate-a");
		expect(useGraphStore.getState().selectedNodeId).toBe("rust:package:crate-a");
	});

	it("deselectNode clears selectedNodeId", () => {
		useGraphStore.setState({ selectedNodeId: "some-id" });
		useGraphStore.getState().deselectNode();
		expect(useGraphStore.getState().selectedNodeId).toBeNull();
	});

	it("selected node data is accessible from discoveredNodes", () => {
		const nodes = [
			makeNode("rust:package:crate-a", "package", "crate-a"),
			makeNode("rust:file:crate-a/src/lib.rs", "file", "lib.rs", "rust:package:crate-a"),
		];
		useGraphStore.setState({ discoveredNodes: nodes });
		useGraphStore.getState().selectNode("rust:file:crate-a/src/lib.rs");

		const state = useGraphStore.getState();
		const selected = state.discoveredNodes.find((n) => n.id === state.selectedNodeId);
		expect(selected).toBeDefined();
		expect(selected?.data.label).toBe("lib.rs");
		expect(selected?.data.kind).toBe("file");
	});

	it("edges for selected node can be computed", () => {
		const nodes = [
			makeNode("rust:file:a.rs", "file", "a.rs"),
			makeNode("rust:file:b.rs", "file", "b.rs"),
			makeNode("rust:file:c.rs", "file", "c.rs"),
		];
		const edges = [
			makeEdge("e1", "rust:file:a.rs", "rust:file:b.rs", "value"),
			makeEdge("e2", "rust:file:c.rs", "rust:file:b.rs", "dev"),
			makeEdge("e3", "rust:file:b.rs", "rust:file:a.rs", "typeOnly"),
		];
		useGraphStore.setState({ discoveredNodes: nodes, discoveredEdges: edges });
		useGraphStore.getState().selectNode("rust:file:b.rs");

		const state = useGraphStore.getState();
		const nodeId = state.selectedNodeId;
		const incoming = state.discoveredEdges.filter((e) => e.target === nodeId);
		const outgoing = state.discoveredEdges.filter((e) => e.source === nodeId);

		expect(incoming).toHaveLength(2);
		expect(outgoing).toHaveLength(1);
		expect(incoming[0]?.data.category).toBe("value");
		expect(incoming[1]?.data.category).toBe("dev");
		expect(outgoing[0]?.data.category).toBe("typeOnly");
	});

	it("unsupported constructs can be filtered by node path", () => {
		const constructs: UnsupportedConstruct[] = [
			{
				constructType: "cfgGate",
				location: { path: "crates/core/src/lib.rs", startLine: 10, endLine: 12 },
				impact: "Module may be missing",
				howToAddress: "Check cfg flags",
			},
			{
				constructType: "dynamicImport",
				location: { path: "src/App.tsx", startLine: 5, endLine: 5 },
				impact: "Import not resolved",
				howToAddress: "Use static import",
			},
		];
		useScanStore.setState({ unsupportedConstructs: constructs });

		const nodePath = "crates/core";
		const filtered = constructs.filter((c) => c.location.path.startsWith(nodePath));
		expect(filtered).toHaveLength(1);
		expect(filtered[0]?.constructType).toBe("cfgGate");
	});

	it("parse failures can be filtered by node path", () => {
		const failures: ParseFailure[] = [
			{ path: "crates/core/src/bad.rs", reason: "Syntax error" },
			{ path: "src/broken.tsx", reason: "JSX error" },
		];
		useScanStore.setState({ parseFailures: failures });

		const nodePath = "crates/core";
		const filtered = failures.filter(
			(f) => f.path === nodePath || f.path.startsWith(`${nodePath}/`),
		);
		expect(filtered).toHaveLength(1);
		expect(filtered[0]?.path).toBe("crates/core/src/bad.rs");
	});
});
