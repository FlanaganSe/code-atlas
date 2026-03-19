/**
 * Graph projection pipeline — pure function that transforms the discovered graph
 * into the visible graph for React Flow.
 *
 * Pipeline: merge → filter by category → handle suppression → collapse projection
 *
 * This is the single source of truth for what React Flow renders.
 */

import type { EdgeCategory, MaterializedKey, NodeKind } from "@/types/graph";

// ---------------------------------------------------------------------------
// AppNode / AppEdge — React Flow compatible types
// ---------------------------------------------------------------------------

export interface AppNodeData {
	readonly label: string;
	readonly kind: NodeKind;
	readonly language: string;
	readonly materializedKey: MaterializedKey;
	readonly parentKey: MaterializedKey | null;
	readonly isExpanded: boolean;
	readonly childCount: number;
	readonly unsupportedConstructs: number;
}

export interface AppNode {
	readonly id: string;
	readonly type: "package" | "module" | "file";
	readonly position: { x: number; y: number };
	readonly data: AppNodeData;
	readonly parentId?: string;
	readonly hidden?: boolean;
	readonly style?: Record<string, string | number>;
}

export interface AppEdgeData {
	readonly category: EdgeCategory;
	readonly isManual: boolean;
	readonly isSuppressed: boolean;
	readonly isBundled: boolean;
	readonly bundledEdgeIds: readonly string[];
	readonly bundledCount: number;
	readonly confidence: string;
	readonly edgeId: string;
}

export interface AppEdge {
	readonly id: string;
	readonly source: string;
	readonly target: string;
	readonly type: "dependency";
	readonly data: AppEdgeData;
	readonly hidden?: boolean;
}

// ---------------------------------------------------------------------------
// Helper: MaterializedKey → string ID
// ---------------------------------------------------------------------------

export function keyToId(key: MaterializedKey): string {
	return `${key.language}:${key.entityKind}:${key.relativePath}`;
}

// ---------------------------------------------------------------------------
// Helper: get all ancestor IDs of a node
// ---------------------------------------------------------------------------

function getAncestorIds(nodeId: string, parentMap: ReadonlyMap<string, string>): Set<string> {
	const ancestors = new Set<string>();
	let current = parentMap.get(nodeId);
	while (current) {
		ancestors.add(current);
		current = parentMap.get(current);
	}
	return ancestors;
}

// ---------------------------------------------------------------------------
// Helper: get all descendant IDs of a node
// ---------------------------------------------------------------------------

function getDescendantIds(nodeId: string, childrenMap: ReadonlyMap<string, string[]>): string[] {
	const descendants: string[] = [];
	const stack = [...(childrenMap.get(nodeId) ?? [])];
	while (stack.length > 0) {
		const id = stack.pop();
		if (!id) continue;
		descendants.push(id);
		const children = childrenMap.get(id);
		if (children) {
			stack.push(...children);
		}
	}
	return descendants;
}

// ---------------------------------------------------------------------------
// Helper: find the nearest visible ancestor (collapsed package) for a node
// ---------------------------------------------------------------------------

function findNearestVisibleAncestor(
	nodeId: string,
	parentMap: ReadonlyMap<string, string>,
	hiddenNodes: ReadonlySet<string>,
): string | null {
	let current = parentMap.get(nodeId);
	while (current) {
		if (!hiddenNodes.has(current)) {
			return current;
		}
		current = parentMap.get(current);
	}
	return null;
}

// ---------------------------------------------------------------------------
// Helper: determine the majority category for a bundled edge
// ---------------------------------------------------------------------------

function majorityCategory(categories: EdgeCategory[]): EdgeCategory {
	const counts = new Map<EdgeCategory, number>();
	for (const cat of categories) {
		counts.set(cat, (counts.get(cat) ?? 0) + 1);
	}
	let maxCount = 0;
	let maxCat: EdgeCategory = "normal";
	for (const [cat, count] of counts) {
		if (count > maxCount) {
			maxCount = count;
			maxCat = cat;
		}
	}
	return maxCat;
}

// ---------------------------------------------------------------------------
// Projection function
// ---------------------------------------------------------------------------

export interface ProjectionInput {
	readonly discoveredNodes: readonly AppNode[];
	readonly discoveredEdges: readonly AppEdge[];
	readonly overlayEdges: readonly AppEdge[];
	readonly suppressedEdgeIds: ReadonlySet<string>;
	readonly expandedNodeIds: ReadonlySet<string>;
	readonly categoryFilter: ReadonlySet<EdgeCategory>;
	readonly showSuppressed: boolean;
}

export interface ProjectionResult {
	readonly nodes: AppNode[];
	readonly edges: AppEdge[];
}

export function project(input: ProjectionInput): ProjectionResult {
	const {
		discoveredNodes,
		discoveredEdges,
		overlayEdges,
		suppressedEdgeIds,
		expandedNodeIds,
		categoryFilter,
		showSuppressed,
	} = input;

	// Step 1: Merge discovered + overlay edges
	const mergedEdges: AppEdge[] = [
		...discoveredEdges.map((e) => ({
			...e,
			data: { ...e.data, isManual: false },
		})),
		...overlayEdges.map((e) => ({
			...e,
			data: { ...e.data, isManual: true },
		})),
	];

	// Step 2: Filter by category
	const categoryFiltered = mergedEdges.filter((e) => categoryFilter.has(e.data.category));

	// Step 3: Handle suppression
	const suppressionFiltered: AppEdge[] = [];
	for (const edge of categoryFiltered) {
		const isSuppressed = suppressedEdgeIds.has(edge.data.edgeId);
		if (isSuppressed && !showSuppressed) {
			continue;
		}
		suppressionFiltered.push({
			...edge,
			data: { ...edge.data, isSuppressed },
		});
	}

	// Step 4: Collapse projection
	// Build parent → children map and parent lookup
	const parentMap = new Map<string, string>();
	const childrenMap = new Map<string, string[]>();

	for (const node of discoveredNodes) {
		if (node.parentId) {
			parentMap.set(node.id, node.parentId);
			const siblings = childrenMap.get(node.parentId) ?? [];
			siblings.push(node.id);
			childrenMap.set(node.parentId, siblings);
		}
	}

	// Determine hidden nodes: descendants of collapsed compound nodes
	const hiddenNodes = new Set<string>();
	for (const node of discoveredNodes) {
		const isCompound = node.type === "package" || node.type === "module";
		if (isCompound && !expandedNodeIds.has(node.id)) {
			const descendants = getDescendantIds(node.id, childrenMap);
			for (const d of descendants) {
				hiddenNodes.add(d);
			}
		}
	}

	// Build projected nodes
	const projectedNodes: AppNode[] = discoveredNodes.map((node) => {
		const isHidden = hiddenNodes.has(node.id);
		const isCompound = node.type === "package" || node.type === "module";
		const isExpanded = isCompound && expandedNodeIds.has(node.id);
		const descendants = getDescendantIds(node.id, childrenMap);
		const childCount = descendants.length;

		// If a node's parent is hidden, this node should also not have a parentId
		// in the projected output (it's hidden anyway)
		const effectiveParentId = node.parentId;

		// Check if parent itself is hidden — if so, the parentId should be remapped
		// to nearest visible ancestor (but since the node is hidden too, it doesn't matter)

		return {
			...node,
			hidden: isHidden,
			data: {
				...node.data,
				isExpanded,
				childCount,
			},
			parentId: effectiveParentId,
		};
	});

	// Build projected edges with bundling
	// For edges where one or both ends are hidden, create bundled edges
	const bundleMap = new Map<
		string,
		{ sourceId: string; targetId: string; edgeIds: string[]; categories: EdgeCategory[] }
	>();

	const projectedEdges: AppEdge[] = [];

	for (const edge of suppressionFiltered) {
		const sourceHidden = hiddenNodes.has(edge.source);
		const targetHidden = hiddenNodes.has(edge.target);

		if (!sourceHidden && !targetHidden) {
			// Both visible — keep edge as-is
			projectedEdges.push(edge);
			continue;
		}

		if (sourceHidden && targetHidden) {
			// Both hidden — check if they share the same collapsed ancestor
			const sourceAncestor = findNearestVisibleAncestor(edge.source, parentMap, hiddenNodes);
			const targetAncestor = findNearestVisibleAncestor(edge.target, parentMap, hiddenNodes);

			if (!sourceAncestor || !targetAncestor) continue;
			if (sourceAncestor === targetAncestor) {
				// Both inside the same collapsed node — hide the edge
				continue;
			}

			// Different collapsed packages — create bundled edge between them
			const bundleKey = `bundle:${sourceAncestor}:${targetAncestor}`;
			const existing = bundleMap.get(bundleKey);
			if (existing) {
				existing.edgeIds.push(edge.data.edgeId);
				existing.categories.push(edge.data.category);
			} else {
				bundleMap.set(bundleKey, {
					sourceId: sourceAncestor,
					targetId: targetAncestor,
					edgeIds: [edge.data.edgeId],
					categories: [edge.data.category],
				});
			}
			continue;
		}

		// One hidden, one visible — bundle to the collapsed ancestor
		const hiddenEnd = sourceHidden ? edge.source : edge.target;
		const visibleEnd = sourceHidden ? edge.target : edge.source;
		const ancestor = findNearestVisibleAncestor(hiddenEnd, parentMap, hiddenNodes);

		if (!ancestor) continue;

		// Check if the visible end IS the ancestor (edge within expanded subtree)
		const visibleAncestors = getAncestorIds(visibleEnd, parentMap);
		if (ancestor === visibleEnd || visibleAncestors.has(ancestor)) {
			// Edge from parent to own descendant — skip when collapsed
			continue;
		}

		const bundleSourceId = sourceHidden ? ancestor : visibleEnd;
		const bundleTargetId = sourceHidden ? visibleEnd : ancestor;
		const bundleKey = `bundle:${bundleSourceId}:${bundleTargetId}`;
		const existing = bundleMap.get(bundleKey);
		if (existing) {
			existing.edgeIds.push(edge.data.edgeId);
			existing.categories.push(edge.data.category);
		} else {
			bundleMap.set(bundleKey, {
				sourceId: bundleSourceId,
				targetId: bundleTargetId,
				edgeIds: [edge.data.edgeId],
				categories: [edge.data.category],
			});
		}
	}

	// Convert bundle map to bundled edges
	for (const [bundleKey, bundle] of bundleMap) {
		const category = majorityCategory(bundle.categories);
		projectedEdges.push({
			id: bundleKey,
			source: bundle.sourceId,
			target: bundle.targetId,
			type: "dependency",
			data: {
				category,
				isManual: false,
				isSuppressed: false,
				isBundled: true,
				bundledEdgeIds: bundle.edgeIds,
				bundledCount: bundle.edgeIds.length,
				confidence: "structural",
				edgeId: bundleKey,
			},
		});
	}

	// Sort nodes: parents before children
	const sortedNodes = sortParentsFirst(projectedNodes);

	return { nodes: sortedNodes, edges: projectedEdges };
}

// ---------------------------------------------------------------------------
// Sort nodes so parents appear before children
// ---------------------------------------------------------------------------

function sortParentsFirst(nodes: AppNode[]): AppNode[] {
	const nodeMap = new Map<string, AppNode>();
	for (const n of nodes) nodeMap.set(n.id, n);

	const depth = new Map<string, number>();

	function getDepth(id: string): number {
		const cached = depth.get(id);
		if (cached !== undefined) return cached;
		const node = nodeMap.get(id);
		if (!node?.parentId) {
			depth.set(id, 0);
			return 0;
		}
		const d = getDepth(node.parentId) + 1;
		depth.set(id, d);
		return d;
	}

	for (const n of nodes) getDepth(n.id);

	return [...nodes].sort((a, b) => {
		const da = depth.get(a.id) ?? 0;
		const db = depth.get(b.id) ?? 0;
		return da - db;
	});
}

// ---------------------------------------------------------------------------
// Graph adaptation: compute initial expanded set
// ---------------------------------------------------------------------------

export function computeInitialExpanded(nodes: readonly AppNode[]): Set<string> {
	const totalNodes = nodes.length;
	const expanded = new Set<string>();

	if (totalNodes < 120) {
		// Small graph: expand top-level packages and modules
		for (const node of nodes) {
			if (node.type === "package" || node.type === "module") {
				expanded.add(node.id);
			}
		}
	} else if (totalNodes <= 250) {
		// Medium: only top-level packages expanded (not modules)
		for (const node of nodes) {
			if (node.type === "package" && !node.parentId) {
				expanded.add(node.id);
			}
		}
	}
	// Large (>250): all collapsed — empty set

	return expanded;
}
