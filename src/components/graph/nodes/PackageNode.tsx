import { Handle, Position } from "@xyflow/react";
import { ChevronDown, ChevronRight, Package } from "lucide-react";
import { memo } from "react";
import type { AppNodeData } from "@/store/graph-projection";
import { useGraphStore } from "@/store/graph-store";

interface PackageNodeProps {
	id: string;
	data: AppNodeData;
}

export const PackageNode = memo(function PackageNode({ id, data }: PackageNodeProps) {
	const toggleExpand = useGraphStore((s) => s.toggleExpand);
	const isExpanded = data.isExpanded;

	return (
		<div
			className={`rounded-lg border-2 ${
				isExpanded ? "border-blue-500/50 bg-blue-950/30" : "border-blue-500/30 bg-blue-950/60"
			} min-w-[200px]`}
		>
			<Handle type="target" position={Position.Top} className="!bg-blue-400" />
			<div className="flex items-center gap-2 px-3 py-2">
				<button
					type="button"
					onClick={() => toggleExpand(id)}
					className="rounded p-0.5 text-blue-400 hover:bg-blue-800/50"
				>
					{isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
				</button>
				<Package size={14} className="text-blue-400" />
				<span className="text-sm font-semibold text-blue-200">{data.label}</span>
				{!isExpanded && data.childCount > 0 && (
					<span className="ml-auto text-xs text-blue-400/60">{data.childCount} items</span>
				)}
				{data.unsupportedConstructs > 0 && (
					<span className="rounded bg-amber-900/50 px-1.5 py-0.5 text-xs text-amber-300">
						{data.unsupportedConstructs} unsupported
					</span>
				)}
			</div>
			<Handle type="source" position={Position.Bottom} className="!bg-blue-400" />
		</div>
	);
});
