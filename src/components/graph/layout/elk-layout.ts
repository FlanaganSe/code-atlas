/**
 * ELK layout orchestration.
 *
 * Converts React Flow nodes/edges ↔ ELK hierarchical format and
 * orchestrates layout via the Web Worker.
 */

import type { ElkExtendedEdge, ElkNode } from "elkjs/lib/elk-api";
import type { AppEdge, AppNode } from "@/store/graph-projection";

// ---------------------------------------------------------------------------
// ELK layout options
// ---------------------------------------------------------------------------

const ROOT_LAYOUT_OPTIONS = {
	"elk.algorithm": "layered",
	"elk.direction": "DOWN",
	"elk.hierarchyHandling": "INCLUDE_CHILDREN",
	"elk.layered.spacing.nodeNodeBetweenLayers": "80",
	"elk.spacing.nodeNode": "40",
	"elk.padding": "[top=40,left=15,bottom=15,right=15]",
	"elk.edgeRouting": "POLYLINE",
};

// Estimated node sizes (POC — no DOM measurement)
const NODE_SIZES: Record<string, { width: number; height: number }> = {
	package: { width: 220, height: 50 },
	module: { width: 180, height: 40 },
	file: { width: 160, height: 36 },
};

// ---------------------------------------------------------------------------
// toElkGraph — flat React Flow → ELK hierarchical
// ---------------------------------------------------------------------------

export function toElkGraph(
	nodes: readonly AppNode[],
	edges: readonly AppEdge[],
	expandedNodeIds: ReadonlySet<string>,
): ElkNode {
	// Only include visible nodes
	const visibleNodes = nodes.filter((n) => !n.hidden);

	// Build id → node lookup
	const nodeMap = new Map<string, AppNode>();
	for (const n of visibleNodes) nodeMap.set(n.id, n);

	// Build parent → children map
	const childrenMap = new Map<string, AppNode[]>();
	const rootNodes: AppNode[] = [];

	for (const node of visibleNodes) {
		if (node.parentId && nodeMap.has(node.parentId)) {
			const siblings = childrenMap.get(node.parentId) ?? [];
			siblings.push(node);
			childrenMap.set(node.parentId, siblings);
		} else {
			rootNodes.push(node);
		}
	}

	// Recursively build ELK node tree
	function buildElkNode(node: AppNode): ElkNode {
		const size = NODE_SIZES[node.type] ?? NODE_SIZES.file;
		const children = childrenMap.get(node.id);
		const isExpanded = expandedNodeIds.has(node.id);

		const elkNode: ElkNode = {
			id: node.id,
			width: isExpanded && children ? undefined : size.width,
			height: isExpanded && children ? undefined : size.height,
			layoutOptions: isExpanded
				? {
						"elk.padding": "[top=40,left=15,bottom=15,right=15]",
					}
				: undefined,
		};

		if (isExpanded && children) {
			elkNode.children = children.map(buildElkNode);
		}

		return elkNode;
	}

	// Build visible edges — only those between visible nodes
	const visibleNodeIds = new Set(visibleNodes.map((n) => n.id));
	const elkEdges: ElkExtendedEdge[] = edges
		.filter((e) => !e.hidden && visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target))
		.map((e) => ({
			id: e.id,
			sources: [e.source],
			targets: [e.target],
		}));

	return {
		id: "root",
		layoutOptions: ROOT_LAYOUT_OPTIONS,
		children: rootNodes.map(buildElkNode),
		edges: elkEdges,
	};
}

// ---------------------------------------------------------------------------
// fromElkGraph — ELK hierarchical → flat React Flow positioned nodes
// ---------------------------------------------------------------------------

export interface PositionedNode {
	readonly id: string;
	readonly position: { x: number; y: number };
	readonly style?: { width: number; height: number };
}

export function fromElkGraph(elkResult: ElkNode): PositionedNode[] {
	const positioned: PositionedNode[] = [];

	function flatten(node: ElkNode): void {
		const pos = { x: node.x ?? 0, y: node.y ?? 0 };
		const hasChildren = node.children && node.children.length > 0;

		const result: PositionedNode = {
			id: node.id,
			position: pos,
			style: hasChildren ? { width: node.width ?? 200, height: node.height ?? 100 } : undefined,
		};

		positioned.push(result);

		if (node.children) {
			for (const child of node.children) {
				flatten(child);
			}
		}
	}

	// Flatten starting from root's children (root is the virtual container)
	if (elkResult.children) {
		for (const child of elkResult.children) {
			flatten(child);
		}
	}

	return positioned;
}

// ---------------------------------------------------------------------------
// applyElkPositions — update AppNodes with ELK layout positions
// ---------------------------------------------------------------------------

export function applyElkPositions(
	nodes: readonly AppNode[],
	positions: readonly PositionedNode[],
): AppNode[] {
	const posMap = new Map<string, PositionedNode>();
	for (const p of positions) posMap.set(p.id, p);

	return nodes.map((node) => {
		const pos = posMap.get(node.id);
		if (!pos) return node;

		return {
			...node,
			position: pos.position,
			style: pos.style
				? {
						width: pos.style.width,
						height: pos.style.height,
					}
				: node.style,
		};
	});
}

// ---------------------------------------------------------------------------
// Layout execution — worker with main-thread fallback
// ---------------------------------------------------------------------------

let worker: Worker | null = null;
let workerFailed = false;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function getWorker(): Worker | null {
	if (workerFailed) return null;
	if (!worker) {
		try {
			worker = new Worker(new URL("./elk.worker.ts", import.meta.url), {
				type: "module",
			});
			worker.addEventListener("error", () => {
				console.warn("ELK Web Worker failed to load, using main-thread fallback");
				workerFailed = true;
				worker = null;
			});
		} catch {
			console.warn("ELK Web Worker creation failed, using main-thread fallback");
			workerFailed = true;
			return null;
		}
	}
	return worker;
}

function layoutViaWorker(elkGraph: ElkNode): Promise<ElkNode> {
	return new Promise((resolve, reject) => {
		const w = getWorker();
		if (!w) {
			reject(new Error("worker unavailable"));
			return;
		}

		const timeout = setTimeout(() => {
			w.removeEventListener("message", handler);
			reject(new Error("worker timeout"));
		}, 5000);

		const handler = (event: MessageEvent) => {
			clearTimeout(timeout);
			w.removeEventListener("message", handler);
			if (event.data.type === "success") {
				resolve(event.data.data);
			} else {
				reject(new Error(event.data.error ?? "worker layout error"));
			}
		};

		w.addEventListener("message", handler);
		w.postMessage(elkGraph);
	});
}

// Main-thread fallback using ELK bundled (lazy-loaded)
let mainThreadElk: { layout: (graph: ElkNode) => Promise<ElkNode> } | null = null;

async function layoutMainThread(elkGraph: ElkNode): Promise<ElkNode> {
	if (!mainThreadElk) {
		const ELK = (await import("elkjs/lib/elk.bundled.js")).default;
		mainThreadElk = new ELK();
	}
	return mainThreadElk.layout(elkGraph);
}

/**
 * Orchestrate ELK layout: build graph → post to worker → flatten → return positioned nodes.
 * Falls back to main-thread ELK if the worker is unavailable.
 * Debounced at 300ms for rapid expand/collapse.
 */
export function layoutGraph(
	nodes: readonly AppNode[],
	edges: readonly AppEdge[],
	expandedNodeIds: ReadonlySet<string>,
): Promise<AppNode[]> {
	return new Promise((resolve) => {
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}

		debounceTimer = setTimeout(async () => {
			try {
				const elkGraph = toElkGraph(nodes, edges, expandedNodeIds);
				let result: ElkNode;
				try {
					result = await layoutViaWorker(elkGraph);
				} catch {
					// Worker failed — fall back to main thread
					result = await layoutMainThread(elkGraph);
				}
				const positions = fromElkGraph(result);
				const positioned = applyElkPositions(nodes, positions);
				resolve(positioned);
			} catch (error) {
				console.error("ELK layout failed:", error);
				resolve([...nodes]);
			}
		}, 300);
	});
}
