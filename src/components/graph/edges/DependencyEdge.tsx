import { BaseEdge, EdgeLabelRenderer, getSmoothStepPath } from "@xyflow/react";
import { memo } from "react";
import type { AppEdgeData } from "@/store/graph-projection";
import type { EdgeCategory } from "@/types/graph";

/**
 * Okabe-Ito palette for edge categories.
 */
const EDGE_COLORS: Record<EdgeCategory, string> = {
	value: "#0072B2",
	typeOnly: "#56B4E9",
	dev: "#E69F00",
	build: "#F0E442",
	normal: "#009E73",
	manual: "#CC79A7",
	test: "#D55E00",
	peer: "#999999",
};

const EDGE_DASH: Record<string, string | undefined> = {
	value: undefined, // solid
	typeOnly: "5,5",
	dev: "2,2",
	build: undefined,
	normal: undefined,
	manual: undefined, // uses stroke-width instead
	test: "8,4",
	peer: "4,4",
};

// Suppressed override
const SUPPRESSED_COLOR = "#999999";
const SUPPRESSED_DASH = "10,5";

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

	const color = isSuppressed ? SUPPRESSED_COLOR : EDGE_COLORS[category];
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
