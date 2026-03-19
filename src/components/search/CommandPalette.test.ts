import { beforeEach, describe, expect, it } from "vitest";
import type { AppNode } from "@/store/graph-projection";
import { useGraphStore } from "@/store/graph-store";

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
			language: (parts[0] ?? "rust") as "rust" | "typescript",
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

describe("CommandPalette search logic", () => {
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
		});
	});

	it("search operates on all discoveredNodes including collapsed", () => {
		const pkgNode = makeNode("rust:package:crate-a", "package", "crate-a");
		const fileNode = makeNode(
			"rust:file:crate-a/src/lib.rs",
			"file",
			"lib.rs",
			"rust:package:crate-a",
		);

		// Package is collapsed — file is not in projectedNodes but IS in discoveredNodes
		useGraphStore.setState({
			discoveredNodes: [pkgNode, fileNode],
			expandedNodeIds: new Set(), // collapsed
		});

		const state = useGraphStore.getState();
		// Search over discoveredNodes (all nodes, not just projected/visible)
		const query = "lib.rs";
		const results = state.discoveredNodes.filter((n) =>
			n.data.label.toLowerCase().includes(query.toLowerCase()),
		);

		expect(results).toHaveLength(1);
		expect(results[0]?.data.label).toBe("lib.rs");
	});

	it("search groups results by node type", () => {
		const nodes = [
			makeNode("rust:package:core", "package", "core"),
			makeNode("rust:module:core/graph", "module", "graph", "rust:package:core"),
			makeNode("rust:file:core/graph/types.rs", "file", "types.rs", "rust:module:core/graph"),
			makeNode("rust:file:core/lib.rs", "file", "lib.rs", "rust:package:core"),
		];
		useGraphStore.setState({ discoveredNodes: nodes });

		const state = useGraphStore.getState();
		const packages = state.discoveredNodes.filter((n) => n.type === "package");
		const modules = state.discoveredNodes.filter((n) => n.type === "module");
		const files = state.discoveredNodes.filter((n) => n.type === "file");

		expect(packages).toHaveLength(1);
		expect(modules).toHaveLength(1);
		expect(files).toHaveLength(2);
	});

	it("expandAncestorsOf expands all parents up to root", () => {
		const nodes = [
			makeNode("rust:package:core", "package", "core"),
			makeNode("rust:module:core/graph", "module", "graph", "rust:package:core"),
			makeNode("rust:file:core/graph/types.rs", "file", "types.rs", "rust:module:core/graph"),
		];
		useGraphStore.getState().loadFixture(nodes, []);

		// Collapse all
		useGraphStore.getState().collapseAll();
		expect(useGraphStore.getState().expandedNodeIds.size).toBe(0);

		// Expand ancestors of the file
		useGraphStore.getState().expandAncestorsOf("rust:file:core/graph/types.rs");

		const expanded = useGraphStore.getState().expandedNodeIds;
		expect(expanded.has("rust:package:core")).toBe(true);
		expect(expanded.has("rust:module:core/graph")).toBe(true);
		// The file itself should NOT be in expanded (it's a leaf)
		expect(expanded.has("rust:file:core/graph/types.rs")).toBe(false);
	});
});
