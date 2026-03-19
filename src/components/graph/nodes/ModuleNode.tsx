import { Handle, Position } from "@xyflow/react";
import { ChevronDown, ChevronRight, Folder } from "lucide-react";
import { memo } from "react";
import type { AppNodeData } from "@/store/graph-projection";
import { useGraphStore } from "@/store/graph-store";

interface ModuleNodeProps {
	id: string;
	data: AppNodeData;
}

export const ModuleNode = memo(function ModuleNode({ id, data }: ModuleNodeProps) {
	const toggleExpand = useGraphStore((s) => s.toggleExpand);
	const isExpanded = data.isExpanded;

	return (
		<div
			className={`relative rounded-md border ${
				isExpanded
					? "h-full w-full border-neutral-600/50 bg-neutral-900/30"
					: "border-neutral-600/50 bg-neutral-800/60"
			} min-w-[170px]`}
		>
			<Handle type="target" position={Position.Top} className="!bg-neutral-400" />
			<div className="flex items-center gap-2 px-3 py-1.5">
				<button
					type="button"
					onClick={() => toggleExpand(id)}
					className="rounded p-0.5 text-neutral-400 hover:bg-neutral-700/50"
				>
					{isExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
				</button>
				<Folder size={12} className="text-neutral-400" />
				<span className="text-xs font-medium text-neutral-300">{data.label}</span>
				{!isExpanded && data.childCount > 0 && (
					<span className="ml-auto text-xs text-neutral-400">{data.childCount}</span>
				)}
			</div>
			<Handle type="source" position={Position.Bottom} className="!bg-neutral-400" />
		</div>
	);
});
