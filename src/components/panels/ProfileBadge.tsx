/**
 * Profile Badge — compact badge showing active graph profile.
 *
 * Always visible. Click to expand showing workspace details,
 * detected packages, config status, entrypoints, and link
 * to full compatibility report.
 *
 * Data source: DiscoveryResult (workspace, profile, config).
 */

import { ChevronDown, ChevronRight, FileCode, Package, Settings } from "lucide-react";
import { memo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import type { DiscoveryResult, Entrypoint } from "@/types/config";

const LANGUAGE_ICONS: Record<string, string> = {
	rust: "🦀",
	typescript: "TS",
	javascript: "JS",
};

const WORKSPACE_KIND_LABELS: Record<string, string> = {
	cargo: "Cargo",
	pnpm: "pnpm",
	npmYarn: "npm/Yarn",
	mixed: "Mixed",
	single: "Single",
};

interface ProfileBadgeProps {
	result: DiscoveryResult;
	onOpenCompatibility: () => void;
}

export const ProfileBadge = memo(function ProfileBadge({
	result,
	onOpenCompatibility,
}: ProfileBadgeProps): React.JSX.Element {
	const [expanded, setExpanded] = useState(false);
	const { profile, workspace, config } = result;

	return (
		<Popover open={expanded} onOpenChange={setExpanded}>
			<PopoverTrigger className="flex items-center gap-2 rounded-md border border-neutral-700 bg-neutral-800/50 px-3 py-1.5 text-xs hover:bg-neutral-700/50">
				{/* Language icons */}
				{profile.languages.map((lang) => (
					<span key={lang} className="font-mono text-[10px] text-neutral-300">
						{LANGUAGE_ICONS[lang] ?? lang}
					</span>
				))}
				<Separator orientation="vertical" className="h-3" />
				{/* Package manager */}
				<span className="text-neutral-400">{profile.packageManager ?? "—"}</span>
				{/* Expand indicator */}
				{expanded ? (
					<ChevronDown className="h-3 w-3 text-neutral-500" />
				) : (
					<ChevronRight className="h-3 w-3 text-neutral-500" />
				)}
			</PopoverTrigger>
			<PopoverContent className="w-80 p-0" align="start" side="bottom" sideOffset={8}>
				<ScrollArea className="max-h-96">
					<div className="space-y-3 p-3">
						{/* Workspace info */}
						<Section title="Workspace">
							<Row label="Root" value={workspace.root} mono />
							<Row label="Type" value={WORKSPACE_KIND_LABELS[workspace.kind] ?? workspace.kind} />
							<Row label="Packages" value={String(workspace.packages.length)} />
						</Section>

						{/* Profile details */}
						<Section title="Profile">
							<Row label="Languages" value={profile.languages.join(", ") || "None"} />
							<Row label="Package manager" value={profile.packageManager ?? "Unknown"} />
							<Row label="Resolution mode" value={profile.resolutionMode ?? "N/A"} />
							{profile.cargoFeatures.length > 0 && (
								<Row label="Cargo features" value={profile.cargoFeatures.join(", ")} />
							)}
						</Section>

						{/* Packages */}
						{workspace.packages.length > 0 && (
							<Section title={`Packages (${workspace.packages.length})`}>
								<div className="max-h-32 space-y-0.5 overflow-y-auto">
									{workspace.packages.map((pkg) => (
										<div
											key={`${pkg.language}:${pkg.relativePath}`}
											className="flex items-center gap-2 text-[11px]"
										>
											<Package className="h-3 w-3 text-neutral-500" />
											<span className="text-neutral-200">{pkg.name}</span>
											<span className="text-neutral-500">{pkg.relativePath}</span>
										</div>
									))}
								</div>
							</Section>
						)}

						{/* Config status */}
						<Section title="Config (.codeatlas.yaml)">
							<Row
								label="Status"
								value={
									config.ignore.length > 0 || config.entrypoints.length > 0
										? "Loaded"
										: "Default (no file)"
								}
							/>
							{config.ignore.length > 0 && (
								<Row label="Ignore patterns" value={String(config.ignore.length)} />
							)}
						</Section>

						{/* Entrypoints */}
						{config.entrypoints.length > 0 && (
							<Section title="Entrypoints">
								{config.entrypoints.map((ep: Entrypoint) => (
									<div key={ep.path} className="flex items-center gap-2 text-[11px]">
										<FileCode className="h-3 w-3 text-neutral-500" />
										<span className="font-mono text-neutral-200">{ep.path}</span>
										<Badge variant="outline" className="text-[9px]">
											{ep.kind}
										</Badge>
									</div>
								))}
							</Section>
						)}

						<Separator />

						{/* Link to compatibility report */}
						<button
							type="button"
							className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-xs text-neutral-300 hover:bg-neutral-800"
							onClick={() => {
								setExpanded(false);
								onOpenCompatibility();
							}}
						>
							<Settings className="h-3.5 w-3.5" />
							View compatibility report
						</button>
					</div>
				</ScrollArea>
			</PopoverContent>
		</Popover>
	);
});

function Section({
	title,
	children,
}: {
	title: string;
	children: React.ReactNode;
}): React.JSX.Element {
	return (
		<div>
			<h4 className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-neutral-500">
				{title}
			</h4>
			<div className="space-y-1">{children}</div>
		</div>
	);
}

function Row({
	label,
	value,
	mono = false,
}: {
	label: string;
	value: string;
	mono?: boolean;
}): React.JSX.Element {
	return (
		<div className="flex justify-between gap-2 text-[11px]">
			<span className="text-neutral-500">{label}</span>
			<span className={`text-right text-neutral-200 ${mono ? "font-mono" : ""}`}>{value}</span>
		</div>
	);
}
