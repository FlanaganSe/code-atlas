/**
 * Zustand store for graph state.
 *
 * Every mutation that changes source data or view state re-runs the projection.
 * ELK layout is triggered separately (async, debounced) via the use-layout hook.
 */

import {
	applyEdgeChanges,
	applyNodeChanges,
	type EdgeChange,
	type NodeChange,
} from "@xyflow/react";
import { create } from "zustand";
import {
	type AppEdge,
	type AppNode,
	computeInitialExpanded,
	project,
} from "@/store/graph-projection";
import type { EdgeCategory } from "@/types/graph";

// All edge categories enabled by default
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

export interface GraphStore {
	// Source data
	discoveredNodes: AppNode[];
	discoveredEdges: AppEdge[];
	overlayEdges: AppEdge[];
	suppressedEdgeIds: Set<string>;

	// View state
	expandedNodeIds: Set<string>;
	categoryFilter: Set<EdgeCategory>;
	showSuppressed: boolean;

	// Projected data (derived)
	projectedNodes: AppNode[];
	projectedEdges: AppEdge[];

	// Layout version — incremented on each projection to trigger ELK layout
	// without creating a dependency cycle (layout updates projectedNodes positions)
	layoutVersion: number;

	// Actions
	loadFixture: (
		nodes: AppNode[],
		edges: AppEdge[],
		overlayEdges?: AppEdge[],
		suppressedEdgeIds?: Set<string>,
	) => void;
	toggleExpand: (nodeId: string) => void;
	expandAll: () => void;
	collapseAll: () => void;
	setCategoryFilter: (categories: Set<EdgeCategory>) => void;
	toggleSuppressed: () => void;

	// React Flow handlers
	onNodesChange: (changes: NodeChange[]) => void;
	onEdgesChange: (changes: EdgeChange[]) => void;
}

function runProjection(state: {
	discoveredNodes: AppNode[];
	discoveredEdges: AppEdge[];
	overlayEdges: AppEdge[];
	suppressedEdgeIds: Set<string>;
	expandedNodeIds: Set<string>;
	categoryFilter: Set<EdgeCategory>;
	showSuppressed: boolean;
}): { projectedNodes: AppNode[]; projectedEdges: AppEdge[] } {
	const result = project({
		discoveredNodes: state.discoveredNodes,
		discoveredEdges: state.discoveredEdges,
		overlayEdges: state.overlayEdges,
		suppressedEdgeIds: state.suppressedEdgeIds,
		expandedNodeIds: state.expandedNodeIds,
		categoryFilter: state.categoryFilter,
		showSuppressed: state.showSuppressed,
	});
	return { projectedNodes: result.nodes, projectedEdges: result.edges };
}

export const useGraphStore = create<GraphStore>()((set, get) => ({
	discoveredNodes: [],
	discoveredEdges: [],
	overlayEdges: [],
	suppressedEdgeIds: new Set(),

	expandedNodeIds: new Set(),
	categoryFilter: new Set(ALL_CATEGORIES),
	showSuppressed: false,

	projectedNodes: [],
	projectedEdges: [],
	layoutVersion: 0,

	loadFixture: (nodes, edges, overlayEdges = [], suppressedEdgeIds = new Set()) => {
		const expandedNodeIds = computeInitialExpanded(nodes);
		const newState = {
			discoveredNodes: nodes,
			discoveredEdges: edges,
			overlayEdges,
			suppressedEdgeIds,
			expandedNodeIds,
			categoryFilter: get().categoryFilter,
			showSuppressed: get().showSuppressed,
		};
		set({
			...newState,
			...runProjection(newState),
			layoutVersion: get().layoutVersion + 1,
		});
	},

	toggleExpand: (nodeId) => {
		const state = get();
		const next = new Set(state.expandedNodeIds);
		if (next.has(nodeId)) {
			next.delete(nodeId);
		} else {
			next.add(nodeId);
		}
		const newState = { ...state, expandedNodeIds: next };
		set({
			expandedNodeIds: next,
			...runProjection(newState),
			layoutVersion: state.layoutVersion + 1,
		});
	},

	expandAll: () => {
		const state = get();
		const allCompound = new Set(
			state.discoveredNodes
				.filter((n) => n.type === "package" || n.type === "module")
				.map((n) => n.id),
		);
		const newState = { ...state, expandedNodeIds: allCompound };
		set({
			expandedNodeIds: allCompound,
			...runProjection(newState),
			layoutVersion: state.layoutVersion + 1,
		});
	},

	collapseAll: () => {
		const state = get();
		const empty = new Set<string>();
		const newState = { ...state, expandedNodeIds: empty };
		set({
			expandedNodeIds: empty,
			...runProjection(newState),
			layoutVersion: state.layoutVersion + 1,
		});
	},

	setCategoryFilter: (categories) => {
		const state = get();
		const newState = { ...state, categoryFilter: categories };
		set({
			categoryFilter: categories,
			...runProjection(newState),
			layoutVersion: state.layoutVersion + 1,
		});
	},

	toggleSuppressed: () => {
		const state = get();
		const next = !state.showSuppressed;
		const newState = { ...state, showSuppressed: next };
		set({
			showSuppressed: next,
			...runProjection(newState),
			layoutVersion: state.layoutVersion + 1,
		});
	},

	onNodesChange: (changes) => {
		set((state) => ({
			projectedNodes: applyNodeChanges(
				changes,
				state.projectedNodes as unknown as Parameters<typeof applyNodeChanges>[1],
			) as unknown as AppNode[],
		}));
	},

	onEdgesChange: (changes) => {
		set((state) => ({
			projectedEdges: applyEdgeChanges(
				changes,
				state.projectedEdges as unknown as Parameters<typeof applyEdgeChanges>[1],
			) as unknown as AppEdge[],
		}));
	},
}));
