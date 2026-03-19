/**
 * Node Detail Panel — right-side sliding panel showing tabbed node information.
 *
 * Tabs: Overview, Dependencies, Health.
 * Slides in from the right when a node is selected. The graph canvas resizes
 * via flex layout (not overlay).
 */

import {
	AlertTriangle,
	ArrowDownLeft,
	ArrowUpRight,
	ChevronRight,
	FileCode,
	Folder,
	Package,
	X,
} from "lucide-react";
import { memo, useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CATEGORY_COLORS, CATEGORY_LABELS } from "@/constants/edge-styles";
import type { AppEdge, AppNode, AppNodeData } from "@/store/graph-projection";
import { useGraphStore } from "@/store/graph-store";
import { useScanStore } from "@/store/scan-store";
import type { EdgeCategory } from "@/types/graph";

const KIND_ICONS: Record<string, React.JSX.Element> = {
	package: <Package size={16} className="text-blue-400" />,
	module: <Folder size={16} className="text-neutral-400" />,
	file: <FileCode size={16} className="text-neutral-500" />,
};

interface DetailPanelProps {
	onNavigateToEdge?: (sourceId: string, targetId: string) => void;
}

export const DetailPanel = memo(function DetailPanel({
	onNavigateToEdge,
}: DetailPanelProps): React.JSX.Element | null {
	const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
	const discoveredNodes = useGraphStore((s) => s.discoveredNodes);
	const discoveredEdges = useGraphStore((s) => s.discoveredEdges);
	const overlayEdges = useGraphStore((s) => s.overlayEdges);
	const selectNode = useGraphStore((s) => s.selectNode);
	const deselectNode = useGraphStore((s) => s.deselectNode);
	const unsupportedConstructs = useScanStore((s) => s.unsupportedConstructs);
	const parseFailures = useScanStore((s) => s.parseFailures);
	const unresolvedImports = useScanStore((s) => s.unresolvedImports);

	const selectedNode = useMemo(
		() => discoveredNodes.find((n) => n.id === selectedNodeId) ?? null,
		[discoveredNodes, selectedNodeId],
	);

	if (!selectedNode) return null;

	const nodeData = selectedNode.data;
	const nodePath = nodeData.materializedKey.relativePath;

	return (
		<div className="flex h-full w-80 shrink-0 flex-col border-l border-neutral-800 bg-neutral-900">
			{/* Header */}
			<div className="flex items-center justify-between border-b border-neutral-800 px-3 py-2">
				<Breadcrumb node={selectedNode} nodes={discoveredNodes} onNavigate={selectNode} />
				<button
					type="button"
					onClick={deselectNode}
					className="rounded p-1 text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
					aria-label="Close detail panel"
				>
					<X size={14} />
				</button>
			</div>

			{/* Node title */}
			<div className="flex items-center gap-2 border-b border-neutral-800 px-3 py-2">
				{KIND_ICONS[nodeData.kind]}
				<span className="text-sm font-semibold text-neutral-100">{nodeData.label}</span>
			</div>

			{/* Tabs */}
			<Tabs defaultValue={0} className="flex min-h-0 flex-1 flex-col">
				<TabsList variant="line" className="w-full shrink-0 px-3">
					<TabsTrigger value={0} className="text-xs">
						Overview
					</TabsTrigger>
					<TabsTrigger value={1} className="text-xs">
						Dependencies
					</TabsTrigger>
					<TabsTrigger value={2} className="text-xs">
						Health
					</TabsTrigger>
				</TabsList>

				<ScrollArea className="min-h-0 flex-1">
					<TabsContent value={0} className="p-3">
						<OverviewTab nodeData={nodeData} />
					</TabsContent>
					<TabsContent value={1} className="p-3">
						<DependenciesTab
							nodeId={selectedNode.id}
							discoveredEdges={discoveredEdges}
							overlayEdges={overlayEdges}
							discoveredNodes={discoveredNodes}
							onSelectNode={selectNode}
							onNavigateToEdge={onNavigateToEdge}
						/>
					</TabsContent>
					<TabsContent value={2} className="p-3">
						<HealthTab
							nodePath={nodePath}
							nodeKind={nodeData.kind}
							unsupportedConstructs={unsupportedConstructs}
							parseFailures={parseFailures}
							unresolvedImports={unresolvedImports}
							discoveredEdges={discoveredEdges}
							overlayEdges={overlayEdges}
							nodeId={selectedNode.id}
						/>
					</TabsContent>
				</ScrollArea>
			</Tabs>
		</div>
	);
});

// ---------------------------------------------------------------------------
// Breadcrumb
// ---------------------------------------------------------------------------

function Breadcrumb({
	node,
	nodes,
	onNavigate,
}: {
	node: AppNode;
	nodes: readonly AppNode[];
	onNavigate: (nodeId: string) => void;
}): React.JSX.Element {
	const ancestors = useMemo(() => {
		const chain: AppNode[] = [];
		const nodeMap = new Map(nodes.map((n) => [n.id, n]));
		let current = node.parentId ? nodeMap.get(node.parentId) : undefined;
		while (current) {
			chain.unshift(current);
			current = current.parentId ? nodeMap.get(current.parentId) : undefined;
		}
		return chain;
	}, [node, nodes]);

	return (
		<div className="flex items-center gap-1 text-[10px] text-neutral-500">
			{ancestors.map((a) => (
				<span key={a.id} className="flex items-center gap-1">
					<button type="button" onClick={() => onNavigate(a.id)} className="hover:text-neutral-300">
						{a.data.label}
					</button>
					<ChevronRight size={10} />
				</span>
			))}
			<span className="text-neutral-300">{node.data.label}</span>
		</div>
	);
}

// ---------------------------------------------------------------------------
// Overview Tab
// ---------------------------------------------------------------------------

function OverviewTab({ nodeData }: { nodeData: AppNodeData }): React.JSX.Element {
	const key = nodeData.materializedKey;
	return (
		<div className="space-y-3 text-xs">
			<Field label="Kind" value={nodeData.kind} />
			<Field label="Language" value={nodeData.language} />
			<Field label="Path" value={key.relativePath} mono />
			<Field label="Key" value={`${key.language}:${key.entityKind}:${key.relativePath}`} mono />
			{nodeData.childCount > 0 && <Field label="Children" value={String(nodeData.childCount)} />}
			{nodeData.unsupportedConstructs > 0 && (
				<div className="flex items-center gap-2">
					<AlertTriangle size={12} className="text-amber-400" />
					<span className="text-amber-400">
						{nodeData.unsupportedConstructs} unsupported construct
						{nodeData.unsupportedConstructs === 1 ? "" : "s"}
					</span>
				</div>
			)}
		</div>
	);
}

function Field({
	label,
	value,
	mono = false,
}: {
	label: string;
	value: string;
	mono?: boolean;
}): React.JSX.Element {
	return (
		<div>
			<span className="text-neutral-500">{label}</span>
			<p className={`mt-0.5 text-neutral-200 ${mono ? "break-all font-mono text-[11px]" : ""}`}>
				{value}
			</p>
		</div>
	);
}

// ---------------------------------------------------------------------------
// Dependencies Tab
// ---------------------------------------------------------------------------

const ALL_CATEGORIES: EdgeCategory[] = [
	"value",
	"typeOnly",
	"dev",
	"build",
	"normal",
	"manual",
	"test",
	"peer",
];

function DependenciesTab({
	nodeId,
	discoveredEdges,
	overlayEdges,
	discoveredNodes,
	onSelectNode,
	onNavigateToEdge,
}: {
	nodeId: string;
	discoveredEdges: readonly AppEdge[];
	overlayEdges: readonly AppEdge[];
	discoveredNodes: readonly AppNode[];
	onSelectNode: (nodeId: string) => void;
	onNavigateToEdge?: (sourceId: string, targetId: string) => void;
}): React.JSX.Element {
	const [hiddenCategories, setHiddenCategories] = useState<Set<EdgeCategory>>(new Set());

	const allEdges = useMemo(
		() => [...discoveredEdges, ...overlayEdges],
		[discoveredEdges, overlayEdges],
	);

	const nodeMap = useMemo(() => new Map(discoveredNodes.map((n) => [n.id, n])), [discoveredNodes]);

	const incoming = useMemo(
		() => allEdges.filter((e) => e.target === nodeId && !hiddenCategories.has(e.data.category)),
		[allEdges, nodeId, hiddenCategories],
	);

	const outgoing = useMemo(
		() => allEdges.filter((e) => e.source === nodeId && !hiddenCategories.has(e.data.category)),
		[allEdges, nodeId, hiddenCategories],
	);

	const allIncoming = useMemo(
		() => allEdges.filter((e) => e.target === nodeId),
		[allEdges, nodeId],
	);
	const allOutgoing = useMemo(
		() => allEdges.filter((e) => e.source === nodeId),
		[allEdges, nodeId],
	);

	function categorySummary(edges: readonly AppEdge[]): string {
		const counts = new Map<EdgeCategory, number>();
		for (const e of edges) {
			counts.set(e.data.category, (counts.get(e.data.category) ?? 0) + 1);
		}
		return [...counts.entries()]
			.map(([cat, count]) => `${count} ${CATEGORY_LABELS[cat].toLowerCase()}`)
			.join(", ");
	}

	function toggleCategory(cat: EdgeCategory): void {
		setHiddenCategories((prev) => {
			const next = new Set(prev);
			if (next.has(cat)) {
				next.delete(cat);
			} else {
				next.add(cat);
			}
			return next;
		});
	}

	return (
		<div className="space-y-3 text-xs">
			{/* Category filter toggles (local to this tab) */}
			<div className="flex flex-wrap gap-1">
				{ALL_CATEGORIES.map((cat) => {
					const active = !hiddenCategories.has(cat);
					return (
						<button
							key={cat}
							type="button"
							onClick={() => toggleCategory(cat)}
							className={`flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] transition-colors ${
								active
									? "bg-neutral-700 text-neutral-200"
									: "bg-transparent text-neutral-500 ring-1 ring-neutral-700"
							}`}
						>
							<span
								className="inline-block h-2 w-2 rounded-full"
								style={{ backgroundColor: CATEGORY_COLORS[cat], opacity: active ? 1 : 0.3 }}
							/>
							{CATEGORY_LABELS[cat]}
						</button>
					);
				})}
			</div>

			{/* Summary */}
			<p className="text-neutral-400">
				{allIncoming.length} incoming ({categorySummary(allIncoming)}) · {allOutgoing.length}{" "}
				outgoing
			</p>

			<Separator />

			{/* Incoming */}
			<div>
				<h4 className="mb-1.5 flex items-center gap-1.5 font-medium text-neutral-300">
					<ArrowDownLeft size={12} />
					Incoming ({incoming.length})
				</h4>
				{incoming.length === 0 ? (
					<p className="text-neutral-500">No incoming edges</p>
				) : (
					<div className="space-y-1">
						{incoming.map((edge) => (
							<EdgeRow
								key={edge.id}
								edge={edge}
								connectedNodeId={edge.source}
								nodeMap={nodeMap}
								onSelectNode={onSelectNode}
								onNavigateToEdge={onNavigateToEdge}
								direction="incoming"
							/>
						))}
					</div>
				)}
			</div>

			<Separator />

			{/* Outgoing */}
			<div>
				<h4 className="mb-1.5 flex items-center gap-1.5 font-medium text-neutral-300">
					<ArrowUpRight size={12} />
					Outgoing ({outgoing.length})
				</h4>
				{outgoing.length === 0 ? (
					<p className="text-neutral-500">No outgoing edges</p>
				) : (
					<div className="space-y-1">
						{outgoing.map((edge) => (
							<EdgeRow
								key={edge.id}
								edge={edge}
								connectedNodeId={edge.target}
								nodeMap={nodeMap}
								onSelectNode={onSelectNode}
								onNavigateToEdge={onNavigateToEdge}
								direction="outgoing"
							/>
						))}
					</div>
				)}
			</div>
		</div>
	);
}

function EdgeRow({
	edge,
	connectedNodeId,
	nodeMap,
	onSelectNode,
	onNavigateToEdge,
	direction,
}: {
	edge: AppEdge;
	connectedNodeId: string;
	nodeMap: ReadonlyMap<string, AppNode>;
	onSelectNode: (nodeId: string) => void;
	onNavigateToEdge?: (sourceId: string, targetId: string) => void;
	direction: "incoming" | "outgoing";
}): React.JSX.Element {
	const connectedNode = nodeMap.get(connectedNodeId);
	const data = edge.data;

	return (
		<div className="rounded px-2 py-1.5 hover:bg-neutral-800/50">
			<div className="flex items-center gap-2">
				<span
					className="inline-block h-2 w-2 shrink-0 rounded-full"
					style={{ backgroundColor: CATEGORY_COLORS[data.category] }}
				/>
				<button
					type="button"
					onClick={() => onSelectNode(connectedNodeId)}
					className="truncate text-left text-neutral-200 hover:text-blue-400"
				>
					{connectedNode?.data.label ?? connectedNodeId}
				</button>
				{data.isManual && (
					<Badge variant="outline" className="border-pink-400/30 text-[9px] text-pink-400">
						manual
					</Badge>
				)}
			</div>
			<div className="mt-0.5 flex items-center gap-2 pl-4 text-[10px] text-neutral-500">
				<span>{CATEGORY_LABELS[data.category]}</span>
				<span>·</span>
				<span>{data.confidence}</span>
				{onNavigateToEdge && (
					<button
						type="button"
						onClick={() =>
							onNavigateToEdge(
								direction === "incoming" ? connectedNodeId : edge.source,
								direction === "incoming" ? edge.target : connectedNodeId,
							)
						}
						className="text-blue-400 hover:text-blue-300"
					>
						Show in graph
					</button>
				)}
			</div>
		</div>
	);
}

// ---------------------------------------------------------------------------
// Health Tab
// ---------------------------------------------------------------------------

function HealthTab({
	nodePath,
	nodeKind,
	unsupportedConstructs,
	parseFailures,
	unresolvedImports,
	discoveredEdges,
	overlayEdges,
	nodeId,
}: {
	nodePath: string;
	nodeKind: string;
	unsupportedConstructs: readonly import("@/types/graph").UnsupportedConstruct[];
	parseFailures: readonly import("@/types/graph").ParseFailure[];
	unresolvedImports: readonly import("@/types/config").UnresolvedImport[];
	discoveredEdges: readonly AppEdge[];
	overlayEdges: readonly AppEdge[];
	nodeId: string;
}): React.JSX.Element {
	// Filter constructs by path prefix
	const nodeConstructs = useMemo(
		() => unsupportedConstructs.filter((c) => c.location.path.startsWith(nodePath)),
		[unsupportedConstructs, nodePath],
	);

	// Filter parse failures by path prefix
	const nodeParseFailures = useMemo(
		() => parseFailures.filter((f) => f.path === nodePath || f.path.startsWith(`${nodePath}/`)),
		[parseFailures, nodePath],
	);

	// Filter unresolved imports by source file path prefix
	const nodeUnresolved = useMemo(
		() =>
			unresolvedImports.filter(
				(u) => u.sourceFile === nodePath || u.sourceFile.startsWith(`${nodePath}/`),
			),
		[unresolvedImports, nodePath],
	);

	// Check overlay involvement
	const allEdges = useMemo(
		() => [...discoveredEdges, ...overlayEdges],
		[discoveredEdges, overlayEdges],
	);
	const manualEdgeCount = allEdges.filter(
		(e) => (e.source === nodeId || e.target === nodeId) && e.data.isManual,
	).length;
	const suppressedEdgeCount = allEdges.filter(
		(e) => (e.source === nodeId || e.target === nodeId) && e.data.isSuppressed,
	).length;

	return (
		<div className="space-y-3 text-xs">
			{/* Unsupported constructs */}
			<div>
				<h4 className="mb-1.5 font-medium text-neutral-300">
					Unsupported Constructs ({nodeConstructs.length})
				</h4>
				{nodeConstructs.length === 0 ? (
					<p className="text-neutral-500">No unsupported constructs in this {nodeKind}.</p>
				) : (
					<div className="space-y-1.5">
						{nodeConstructs.map((c) => (
							<div
								key={`${c.location.path}:${c.location.startLine}`}
								className="rounded bg-neutral-800/50 px-2 py-1.5"
							>
								<div className="flex items-center gap-2">
									<AlertTriangle size={10} className="text-amber-400" />
									<span className="text-amber-300">{c.constructType}</span>
								</div>
								<p className="mt-0.5 pl-4 text-neutral-400">{c.impact}</p>
								<p className="mt-0.5 pl-4 font-mono text-[10px] text-neutral-500">
									{c.location.path}:{c.location.startLine}
								</p>
							</div>
						))}
					</div>
				)}
			</div>

			<Separator />

			{/* Unresolved imports */}
			<div>
				<h4 className="mb-1.5 font-medium text-neutral-300">
					Unresolved Imports ({nodeUnresolved.length})
				</h4>
				{nodeUnresolved.length === 0 ? (
					<p className="text-neutral-500">No unresolved imports in this {nodeKind}.</p>
				) : (
					<div className="space-y-1.5">
						{nodeUnresolved.map((u, i) => (
							<div
								key={`${u.sourceFile}:${u.specifier}:${i}`}
								className="rounded bg-neutral-800/50 px-2 py-1.5"
							>
								<div className="flex items-center gap-2">
									<AlertTriangle size={10} className="text-orange-400" />
									<span className="font-mono text-orange-300">{u.specifier}</span>
								</div>
								<p className="mt-0.5 pl-4 text-neutral-400">{u.reason.type}</p>
								<p className="mt-0.5 pl-4 font-mono text-[10px] text-neutral-500">{u.sourceFile}</p>
							</div>
						))}
					</div>
				)}
			</div>

			<Separator />

			{/* Parse failures */}
			<div>
				<h4 className="mb-1.5 font-medium text-neutral-300">
					Parse Failures ({nodeParseFailures.length})
				</h4>
				{nodeParseFailures.length === 0 ? (
					<p className="text-neutral-500">No parse failures.</p>
				) : (
					<div className="space-y-1">
						{nodeParseFailures.map((f) => (
							<div key={f.path} className="rounded bg-neutral-800/50 px-2 py-1.5">
								<span className="font-mono text-neutral-200">{f.path}</span>
								<p className="mt-0.5 text-neutral-400">{f.reason}</p>
							</div>
						))}
					</div>
				)}
			</div>

			<Separator />

			{/* Overlay status */}
			<div>
				<h4 className="mb-1.5 font-medium text-neutral-300">Overlay Status</h4>
				{manualEdgeCount === 0 && suppressedEdgeCount === 0 ? (
					<p className="text-neutral-500">No overlay involvement.</p>
				) : (
					<div className="space-y-1">
						{manualEdgeCount > 0 && (
							<p className="text-pink-400">
								{manualEdgeCount} manual edge{manualEdgeCount === 1 ? "" : "s"}
							</p>
						)}
						{suppressedEdgeCount > 0 && (
							<p className="text-neutral-400">
								{suppressedEdgeCount} suppressed edge{suppressedEdgeCount === 1 ? "" : "s"}
							</p>
						)}
					</div>
				)}
			</div>
		</div>
	);
}
