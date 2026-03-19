/**
 * Command Palette — Cmd+K fuzzy search over all graph nodes.
 *
 * Uses shadcn/ui Command (wraps cmdk) for fuzzy search and keyboard navigation.
 * Searches all nodes including those inside collapsed packages.
 * Selecting a result: expands ancestors → re-projects → layouts → centers → selects.
 */

import { FileCode, Folder, Package } from "lucide-react";
import { memo, useCallback, useMemo } from "react";
import {
	Command,
	CommandDialog,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
} from "@/components/ui/command";
import { useGraphStore } from "@/store/graph-store";

const KIND_ICONS: Record<string, React.JSX.Element> = {
	package: <Package size={14} className="text-blue-400" />,
	module: <Folder size={14} className="text-neutral-400" />,
	file: <FileCode size={14} className="text-neutral-500" />,
};

const LANGUAGE_BADGES: Record<string, string> = {
	rust: "RS",
	typescript: "TS",
	javascript: "JS",
};

interface CommandPaletteProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onNavigateToNode: (nodeId: string) => void;
}

export const CommandPalette = memo(function CommandPalette({
	open,
	onOpenChange,
	onNavigateToNode,
}: CommandPaletteProps): React.JSX.Element {
	const discoveredNodes = useGraphStore((s) => s.discoveredNodes);

	// Group nodes by kind
	const grouped = useMemo(() => {
		const packages = discoveredNodes.filter((n) => n.type === "package");
		const modules = discoveredNodes.filter((n) => n.type === "module");
		const files = discoveredNodes.filter((n) => n.type === "file");
		return { packages, modules, files };
	}, [discoveredNodes]);

	// Find parent label for display
	const parentLabels = useMemo(() => {
		const map = new Map<string, string>();
		const nodeMap = new Map(discoveredNodes.map((n) => [n.id, n]));
		for (const node of discoveredNodes) {
			if (node.parentId) {
				const parent = nodeMap.get(node.parentId);
				if (parent) {
					map.set(node.id, parent.data.label);
				}
			}
		}
		return map;
	}, [discoveredNodes]);

	const handleSelect = useCallback(
		(nodeId: string) => {
			onOpenChange(false);
			onNavigateToNode(nodeId);
		},
		[onOpenChange, onNavigateToNode],
	);

	return (
		<CommandDialog
			open={open}
			onOpenChange={onOpenChange}
			title="Search nodes"
			description="Search for packages, modules, and files in the graph"
		>
			<Command className="rounded-xl border border-neutral-700 bg-neutral-900">
				<CommandInput placeholder="Search nodes..." />
				<CommandList>
					<CommandEmpty>No results found.</CommandEmpty>

					{grouped.packages.length > 0 && (
						<CommandGroup heading="Packages">
							{grouped.packages.map((node) => (
								<CommandItem
									key={node.id}
									value={`${node.data.label} ${node.data.materializedKey.relativePath}`}
									onSelect={() => handleSelect(node.id)}
								>
									{KIND_ICONS[node.type]}
									<span className="flex-1 truncate">{node.data.label}</span>
									{LANGUAGE_BADGES[node.data.language] && (
										<span className="rounded bg-neutral-700 px-1.5 py-0.5 text-[9px] font-mono text-neutral-400">
											{LANGUAGE_BADGES[node.data.language]}
										</span>
									)}
								</CommandItem>
							))}
						</CommandGroup>
					)}

					{grouped.modules.length > 0 && (
						<CommandGroup heading="Modules">
							{grouped.modules.map((node) => (
								<CommandItem
									key={node.id}
									value={`${node.data.label} ${node.data.materializedKey.relativePath}`}
									onSelect={() => handleSelect(node.id)}
								>
									{KIND_ICONS[node.type]}
									<span className="flex-1 truncate">{node.data.label}</span>
									{parentLabels.get(node.id) && (
										<span className="text-[10px] text-neutral-500">
											{parentLabels.get(node.id)}
										</span>
									)}
								</CommandItem>
							))}
						</CommandGroup>
					)}

					{grouped.files.length > 0 && (
						<CommandGroup heading="Files">
							{grouped.files.map((node) => (
								<CommandItem
									key={node.id}
									value={`${node.data.label} ${node.data.materializedKey.relativePath}`}
									onSelect={() => handleSelect(node.id)}
								>
									{KIND_ICONS[node.type]}
									<span className="flex-1 truncate">{node.data.label}</span>
									{parentLabels.get(node.id) && (
										<span className="text-[10px] text-neutral-500">
											{parentLabels.get(node.id)}
										</span>
									)}
									{LANGUAGE_BADGES[node.data.language] && (
										<span className="rounded bg-neutral-700 px-1.5 py-0.5 text-[9px] font-mono text-neutral-400">
											{LANGUAGE_BADGES[node.data.language]}
										</span>
									)}
								</CommandItem>
							))}
						</CommandGroup>
					)}
				</CommandList>
			</Command>
		</CommandDialog>
	);
});
