import {
	Background,
	Controls,
	type EdgeMouseHandler,
	MiniMap,
	ReactFlow,
	ReactFlowProvider,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useCallback, useState } from "react";

import { useLayout } from "@/hooks/use-layout";
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

	// Edge provenance popover state
	const [provenancePos, setProvenancePos] = useState<{ x: number; y: number } | null>(null);
	const [provenanceData, setProvenanceData] = useState<AppEdgeData | null>(null);

	useLayout();

	const handleEdgeClick: EdgeMouseHandler = useCallback((event, edge) => {
		const data = edge.data as AppEdgeData | undefined;
		if (!data) return;
		setProvenancePos({ x: event.clientX, y: event.clientY });
		setProvenanceData(data);
	}, []);

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
				onEdgeClick={handleEdgeClick}
				onPaneClick={handleCloseProvenance}
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
