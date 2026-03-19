import { Handle, Position } from "@xyflow/react";
import { AlertTriangle, FileCode } from "lucide-react";
import { memo } from "react";
import type { AppNodeData } from "@/store/graph-projection";

interface FileNodeProps {
	data: AppNodeData;
}

export const FileNode = memo(function FileNode({ data }: FileNodeProps) {
	return (
		<div className="rounded border border-neutral-700/50 bg-neutral-800/80 min-w-[140px]">
			<Handle type="target" position={Position.Top} className="!bg-neutral-500" />
			<div className="flex items-center gap-2 px-3 py-1.5">
				<FileCode size={12} className="text-neutral-500" />
				<span className="text-xs text-neutral-300">{data.label}</span>
				{data.unsupportedConstructs > 0 && (
					<AlertTriangle size={12} className="ml-auto text-amber-400" />
				)}
			</div>
			<Handle type="source" position={Position.Bottom} className="!bg-neutral-500" />
		</div>
	);
});
