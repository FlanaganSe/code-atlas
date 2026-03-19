import {
	Background,
	Controls,
	type EdgeMouseHandler,
	MiniMap,
	type NodeMouseHandler,
	ReactFlow,
	ReactFlowProvider,
	useReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useCallback, useEffect, useRef, useState } from "react";

import { useLayout } from "@/hooks/use-layout";
import { registerViewportFns } from "@/hooks/viewport-ref";
import type { AppEdgeData } from "@/store/graph-projection";
import { useGraphStore } from "@/store/graph-store";
import { EdgeProvenancePopover } from "../panels/EdgeProvenance";
import { DependencyEdge } from "./edges/DependencyEdge";
import { FileNode } from "./nodes/FileNode";
import { ModuleNode } from "./nodes/ModuleNode";
import { PackageNode } from "./nodes/PackageNode";

// Defined outside component to avoid re-creation on render (no React Compiler)
const nodeTypes = {
	package: PackageNode,
	module: ModuleNode,
	file: FileNode,
};

const edgeTypes = {
	dependency: DependencyEdge,
};

function GraphCanvasInner(): React.JSX.Element {
	const nodes = useGraphStore((s) => s.projectedNodes);
	const edges = useGraphStore((s) => s.projectedEdges);
	const onNodesChange = useGraphStore((s) => s.onNodesChange);
	const onEdgesChange = useGraphStore((s) => s.onEdgesChange);
	const selectNode = useGraphStore((s) => s.selectNode);
	const deselectNode = useGraphStore((s) => s.deselectNode);
	const toggleExpand = useGraphStore((s) => s.toggleExpand);
	const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
	const pendingViewport = useGraphStore((s) => s.pendingViewport);
	const setPendingViewport = useGraphStore((s) => s.setPendingViewport);

	const reactFlowInstance = useReactFlow();

	// Register viewport functions for use outside ReactFlowProvider
	useEffect(() => {
		registerViewportFns({
			getViewport: () => reactFlowInstance.getViewport(),
			fitView: (options) => reactFlowInstance.fitView(options),
		});
	}, [reactFlowInstance]);

	// Edge provenance popover state
	const [provenancePos, setProvenancePos] = useState<{ x: number; y: number } | null>(null);
	const [provenanceData, setProvenanceData] = useState<AppEdgeData | null>(null);

	useLayout();

	// Restore pending viewport after layout settles
	const viewportTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	useEffect(() => {
		if (pendingViewport && nodes.length > 0) {
			if (viewportTimerRef.current) clearTimeout(viewportTimerRef.current);
			viewportTimerRef.current = setTimeout(() => {
				reactFlowInstance.setViewport(pendingViewport, { duration: 0 });
				setPendingViewport(null);
			}, 600);
		}
		return () => {
			if (viewportTimerRef.current) clearTimeout(viewportTimerRef.current);
		};
	}, [pendingViewport, nodes.length, reactFlowInstance, setPendingViewport]);

	const handleNodeClick: NodeMouseHandler = useCallback(
		(_event, node) => {
			selectNode(node.id);
		},
		[selectNode],
	);

	const handleNodeDoubleClick: NodeMouseHandler = useCallback(
		(_event, node) => {
			if (node.type === "package" || node.type === "module") {
				toggleExpand(node.id);
			}
		},
		[toggleExpand],
	);

	const handleEdgeClick: EdgeMouseHandler = useCallback((event, edge) => {
		const data = edge.data as AppEdgeData | undefined;
		if (!data) return;
		setProvenancePos({ x: event.clientX, y: event.clientY });
		setProvenanceData(data);
	}, []);

	const handlePaneClick = useCallback(() => {
		setProvenancePos(null);
		setProvenanceData(null);
		deselectNode();
	}, [deselectNode]);

	const handleCloseProvenance = useCallback(() => {
		setProvenancePos(null);
		setProvenanceData(null);
	}, []);

	return (
		<div className="h-full w-full">
			<ReactFlow
				nodes={nodes as unknown as Parameters<typeof ReactFlow>[0]["nodes"]}
				edges={edges as unknown as Parameters<typeof ReactFlow>[0]["edges"]}
				onNodesChange={onNodesChange}
				onEdgesChange={onEdgesChange}
				onNodeClick={handleNodeClick}
				onNodeDoubleClick={handleNodeDoubleClick}
				onEdgeClick={handleEdgeClick}
				onPaneClick={handlePaneClick}
				nodeTypes={nodeTypes}
				edgeTypes={edgeTypes}
				fitView
				onlyRenderVisibleElements
				proOptions={{ hideAttribution: true }}
				colorMode="dark"
			>
				<Background gap={16} size={1} color="#333" />
				<Controls />
				<MiniMap
					zoomable
					pannable
					nodeColor={(node) => {
						if (node.id === selectedNodeId) return "#6366f1";
						switch (node.type) {
							case "package":
								return "#1e40af";
							case "module":
								return "#525252";
							case "file":
								return "#737373";
							default:
								return "#666";
						}
					}}
				/>
			</ReactFlow>
			<EdgeProvenancePopover
				position={provenancePos}
				edgeData={provenanceData}
				onClose={handleCloseProvenance}
			/>
		</div>
	);
}

export function GraphCanvas(): React.JSX.Element {
	return (
		<ReactFlowProvider>
			<GraphCanvasInner />
		</ReactFlowProvider>
	);
}
