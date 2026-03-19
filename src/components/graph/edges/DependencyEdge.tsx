import { BaseEdge, EdgeLabelRenderer, getSmoothStepPath } from "@xyflow/react";
import { memo } from "react";
import {
	CATEGORY_COLORS,
	EDGE_DASH,
	SUPPRESSED_COLOR,
	SUPPRESSED_DASH,
} from "@/constants/edge-styles";
import type { AppEdgeData } from "@/store/graph-projection";

interface DependencyEdgeProps {
	id: string;
	sourceX: number;
	sourceY: number;
	targetX: number;
	targetY: number;
	data?: AppEdgeData;
}

export const DependencyEdge = memo(function DependencyEdge({
	id,
	sourceX,
	sourceY,
	targetX,
	targetY,
	data,
}: DependencyEdgeProps) {
	const category = data?.category ?? "normal";
	const isSuppressed = data?.isSuppressed ?? false;
	const isBundled = data?.isBundled ?? false;
	const isManual = data?.isManual ?? false;
	const bundledCount = data?.bundledCount ?? 0;

	const color = isSuppressed ? SUPPRESSED_COLOR : CATEGORY_COLORS[category];
	const dash = isSuppressed ? SUPPRESSED_DASH : EDGE_DASH[category];
	const strokeWidth = isManual ? 3 : isBundled ? 2 : 1.5;

	const [edgePath, labelX, labelY] = getSmoothStepPath({
		sourceX,
		sourceY,
		targetX,
		targetY,
	});

	return (
		<>
			<BaseEdge
				id={id}
				path={edgePath}
				style={{
					stroke: color,
					strokeWidth,
					strokeDasharray: dash,
					opacity: isSuppressed ? 0.4 : 1,
				}}
			/>
			{isBundled && bundledCount > 0 && (
				<EdgeLabelRenderer>
					<div
						style={{
							position: "absolute",
							transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
							pointerEvents: "all",
						}}
						className="rounded-full bg-neutral-800 px-1.5 py-0.5 text-[10px] font-medium text-neutral-300 border border-neutral-600"
					>
						{bundledCount} {bundledCount === 1 ? "import" : "imports"}
					</div>
				</EdgeLabelRenderer>
			)}
		</>
	);
});
