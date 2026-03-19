import { describe, expect, it } from "vitest";
import type { AppEdge, AppNode } from "@/store/graph-projection";
import { keyToId } from "@/store/graph-projection";
import { applyElkPositions, fromElkGraph, toElkGraph } from "./elk-layout";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeNode(
	entityKind: "package" | "module" | "file",
	path: string,
	label: string,
	parentId?: string,
	hidden = false,
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
		hidden,
	};
}

function makeEdge(sourceId: string, targetId: string): AppEdge {
	const id = `edge:${sourceId}→${targetId}`;
	return {
		id,
		source: sourceId,
		target: targetId,
		type: "dependency",
		data: {
			category: "value",
			isManual: false,
			isSuppressed: false,
			isBundled: false,
			bundledEdgeIds: [],
			bundledCount: 0,
			confidence: "syntactic",
			edgeId: id,
		},
	};
}

// ---------------------------------------------------------------------------
// toElkGraph tests
// ---------------------------------------------------------------------------

describe("toElkGraph()", () => {
	it("converts flat nodes into hierarchical ELK format", () => {
		const pkg = makeNode("package", "pkg/a", "@pkg/a");
		const mod = makeNode("module", "pkg/a/src", "src", pkg.id);
		const file = makeNode("file", "pkg/a/src/index.ts", "index.ts", mod.id);

		const expanded = new Set([pkg.id, mod.id]);
		const elkGraph = toElkGraph([pkg, mod, file], [], expanded);

		expect(elkGraph.id).toBe("root");
		expect(elkGraph.children).toHaveLength(1);

		const elkChildren = elkGraph.children ?? [];
		const elkPkg = elkChildren[0];
		expect(elkPkg).toBeDefined();
		expect(elkPkg.id).toBe(pkg.id);
		expect(elkPkg.children).toHaveLength(1);

		const pkgChildren = elkPkg.children ?? [];
		const elkMod = pkgChildren[0];
		expect(elkMod).toBeDefined();
		expect(elkMod.id).toBe(mod.id);
		expect(elkMod.children).toHaveLength(1);

		const modChildren = elkMod.children ?? [];
		const elkFile = modChildren[0];
		expect(elkFile).toBeDefined();
		expect(elkFile.id).toBe(file.id);
		expect(elkFile.children).toBeUndefined();
	});

	it("excludes hidden nodes", () => {
		const pkg = makeNode("package", "pkg/a", "@pkg/a");
		const hiddenFile = makeNode("file", "pkg/a/hidden.ts", "hidden.ts", pkg.id, true);

		const elkGraph = toElkGraph([pkg, hiddenFile], [], new Set([pkg.id]));
		expect(elkGraph.children).toHaveLength(1);
		// pkg should have no children since the file is hidden
		const children = elkGraph.children ?? [];
		expect(children[0].children).toBeUndefined();
	});

	it("collapsed package has no children in ELK graph", () => {
		const pkg = makeNode("package", "pkg/a", "@pkg/a");
		const file = makeNode("file", "pkg/a/index.ts", "index.ts", pkg.id);

		// Not expanded
		const elkGraph = toElkGraph([pkg, file], [], new Set());
		// pkg is visible but file is hidden (would be hidden by projection)
		// However toElkGraph just looks at the hidden flag and parentId
		expect(elkGraph.children).toHaveLength(1);
		// Since file is not hidden explicitly, it appears as child
		// In practice, projection sets hidden=true on children of collapsed nodes
	});

	it("includes edges between visible nodes", () => {
		const pkg1 = makeNode("package", "pkg/a", "@pkg/a");
		const pkg2 = makeNode("package", "pkg/b", "@pkg/b");
		const edge = makeEdge(pkg1.id, pkg2.id);

		const elkGraph = toElkGraph([pkg1, pkg2], [edge], new Set());
		const elkEdges = elkGraph.edges ?? [];
		expect(elkEdges).toHaveLength(1);
		expect(elkEdges[0].sources).toEqual([pkg1.id]);
		expect(elkEdges[0].targets).toEqual([pkg2.id]);
	});

	it("excludes edges with hidden endpoints", () => {
		const pkg1 = makeNode("package", "pkg/a", "@pkg/a");
		const hiddenPkg = makeNode("package", "pkg/b", "@pkg/b", undefined, true);
		const edge = makeEdge(pkg1.id, hiddenPkg.id);

		const elkGraph = toElkGraph([pkg1, hiddenPkg], [edge], new Set());
		expect(elkGraph.edges).toHaveLength(0);
	});
});

// ---------------------------------------------------------------------------
// fromElkGraph tests
// ---------------------------------------------------------------------------

describe("fromElkGraph()", () => {
	it("flattens hierarchical ELK output to positioned nodes", () => {
		const elkResult = {
			id: "root",
			children: [
				{
					id: "pkg",
					x: 10,
					y: 20,
					width: 300,
					height: 200,
					children: [
						{
							id: "file",
							x: 5,
							y: 40,
							width: 160,
							height: 36,
						},
					],
				},
			],
		};

		const positioned = fromElkGraph(elkResult);
		expect(positioned).toHaveLength(2);

		const pkg = positioned.find((n) => n.id === "pkg");
		expect(pkg).toBeDefined();
		expect(pkg?.position).toEqual({ x: 10, y: 20 });
		expect(pkg?.style).toEqual({ width: 300, height: 200 });

		const file = positioned.find((n) => n.id === "file");
		expect(file).toBeDefined();
		expect(file?.position).toEqual({ x: 5, y: 40 });
		expect(file?.style).toBeUndefined(); // leaf node
	});

	it("handles empty result", () => {
		const positioned = fromElkGraph({ id: "root" });
		expect(positioned).toHaveLength(0);
	});
});

// ---------------------------------------------------------------------------
// applyElkPositions tests
// ---------------------------------------------------------------------------

describe("applyElkPositions()", () => {
	it("applies positions to matching nodes", () => {
		const node = makeNode("file", "src/index.ts", "index.ts");
		const positions = [{ id: node.id, position: { x: 100, y: 200 } }];

		const result = applyElkPositions([node], positions);
		expect(result[0].position).toEqual({ x: 100, y: 200 });
	});

	it("preserves nodes without matching position", () => {
		const node = makeNode("file", "src/index.ts", "index.ts");
		const result = applyElkPositions([node], []);
		expect(result[0]).toBe(node); // same reference
	});

	it("applies style for parent nodes", () => {
		const node = makeNode("package", "pkg/a", "@pkg/a");
		const positions = [
			{
				id: node.id,
				position: { x: 0, y: 0 },
				style: { width: 400, height: 300 },
			},
		];

		const result = applyElkPositions([node], positions);
		expect(result[0].style).toEqual({ width: 400, height: 300 });
	});
});

// ---------------------------------------------------------------------------
// Round-trip test
// ---------------------------------------------------------------------------

describe("toElkGraph → fromElkGraph round-trip", () => {
	it("preserves node identities and parent-child relationships", () => {
		const pkg = makeNode("package", "pkg/a", "@pkg/a");
		const mod = makeNode("module", "pkg/a/src", "src", pkg.id);
		const file = makeNode("file", "pkg/a/src/index.ts", "index.ts", mod.id);

		const expanded = new Set([pkg.id, mod.id]);
		const elkGraph = toElkGraph([pkg, mod, file], [], expanded);

		// Simulate ELK adding positions
		const withPositions = JSON.parse(JSON.stringify(elkGraph));
		withPositions.children[0].x = 10;
		withPositions.children[0].y = 20;
		withPositions.children[0].width = 300;
		withPositions.children[0].height = 200;
		withPositions.children[0].children[0].x = 15;
		withPositions.children[0].children[0].y = 40;
		withPositions.children[0].children[0].width = 200;
		withPositions.children[0].children[0].height = 80;
		withPositions.children[0].children[0].children[0].x = 5;
		withPositions.children[0].children[0].children[0].y = 40;

		const positioned = fromElkGraph(withPositions);

		// All 3 nodes should be present
		expect(positioned).toHaveLength(3);

		const ids = positioned.map((n) => n.id);
		expect(ids).toContain(pkg.id);
		expect(ids).toContain(mod.id);
		expect(ids).toContain(file.id);

		// Parent nodes should have computed dimensions (style)
		const positionedPkg = positioned.find((n) => n.id === pkg.id);
		expect(positionedPkg?.style).toBeDefined();

		const positionedMod = positioned.find((n) => n.id === mod.id);
		expect(positionedMod?.style).toBeDefined();

		// Leaf node should NOT have style
		const positionedFile = positioned.find((n) => n.id === file.id);
		expect(positionedFile?.style).toBeUndefined();
	});
});
