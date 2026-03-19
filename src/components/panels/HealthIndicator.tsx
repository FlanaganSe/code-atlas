/**
 * Health Indicator Bar — persistent bar showing at-a-glance graph health.
 *
 * Data source: GraphHealth from scanStore.graphHealth, plus
 * unsupportedConstructs and parseFailures for detail views.
 */

import { AlertTriangle, CheckCircle, FileWarning, Puzzle, XCircle } from "lucide-react";
import { memo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { useScanStore } from "@/store/scan-store";
import type { ParseFailure, UnsupportedConstruct, UnsupportedConstructType } from "@/types/graph";

/** Human-readable labels for unsupported construct types. */
const CONSTRUCT_LABELS: Record<UnsupportedConstructType, string> = {
	cfgGate: "cfg gates",
	buildScript: "build.rs",
	procMacro: "proc macros",
	dynamicImport: "dynamic imports",
	frameworkConvention: "framework conventions",
	exportsCondition: "exports conditions",
	includeMacro: "include! macros",
	commonJsRequire: "CommonJS require()",
	projectReferences: "project references",
	yarnPnp: "Yarn PnP",
};

/** Explanations for unsupported construct types. */
const CONSTRUCT_EXPLANATIONS: Record<UnsupportedConstructType, string> = {
	cfgGate: "cfg-gated modules are not evaluated in this profile",
	buildScript: "build.rs generates code that cannot be statically analyzed",
	procMacro: "Procedural macros generate code at compile time",
	dynamicImport: "Dynamic import() calls cannot be statically resolved",
	frameworkConvention: "Framework-specific conventions are not modeled",
	exportsCondition: "package.json exports conditions are not evaluated in POC",
	includeMacro: "include!() macro loads content from another file",
	commonJsRequire: "CommonJS require() calls are not resolved in POC",
	projectReferences: "TypeScript project references are not followed",
	yarnPnp: "Yarn PnP resolution is not supported",
};

function healthColor(resolvedPct: number): string {
	if (resolvedPct >= 95) return "text-green-400";
	if (resolvedPct >= 80) return "text-amber-400";
	return "text-red-400";
}

function healthIcon(resolvedPct: number): React.JSX.Element {
	if (resolvedPct >= 95) return <CheckCircle className="h-3.5 w-3.5 text-green-400" />;
	if (resolvedPct >= 80) return <AlertTriangle className="h-3.5 w-3.5 text-amber-400" />;
	return <XCircle className="h-3.5 w-3.5 text-red-400" />;
}

export const HealthIndicator = memo(function HealthIndicator(): React.JSX.Element | null {
	const graphHealth = useScanStore((s) => s.graphHealth);
	const unsupportedConstructs = useScanStore((s) => s.unsupportedConstructs);
	const parseFailures = useScanStore((s) => s.parseFailures);
	const scanStatus = useScanStore((s) => s.scanStatus);

	if (!graphHealth || scanStatus === "idle") return null;

	const totalImports = graphHealth.resolvedEdges + graphHealth.unresolvedImports;
	const resolvedPct = totalImports > 0 ? (graphHealth.resolvedEdges / totalImports) * 100 : 100;

	return (
		<div className="flex items-center gap-4 border-b border-neutral-800 bg-neutral-900/80 px-4 py-1.5 text-xs">
			{/* Resolution completeness */}
			<div className="flex items-center gap-1.5">
				{healthIcon(resolvedPct)}
				<span className={healthColor(resolvedPct)}>
					{graphHealth.resolvedEdges} of {totalImports} imports resolved ({resolvedPct.toFixed(1)}%)
				</span>
			</div>

			<Separator orientation="vertical" className="h-4" />

			{/* Parse failures */}
			<ParseFailuresPopover failures={parseFailures} />

			<Separator orientation="vertical" className="h-4" />

			{/* Unsupported constructs */}
			<UnsupportedConstructsPopover constructs={unsupportedConstructs} />

			<Separator orientation="vertical" className="h-4" />

			{/* Total nodes */}
			<span className="text-neutral-400">{graphHealth.totalNodes} nodes</span>
		</div>
	);
});

/** Popover showing parse failure details. */
function ParseFailuresPopover({
	failures,
}: {
	failures: readonly ParseFailure[];
}): React.JSX.Element {
	const [open, setOpen] = useState(false);
	const count = failures.length;

	const trigger = (
		<span className={count > 0 ? "text-amber-400" : "text-neutral-400"}>
			<FileWarning className="mr-1.5 inline h-3.5 w-3.5 text-neutral-400" />
			{count} parse {count === 1 ? "failure" : "failures"}
		</span>
	);

	if (count === 0) return <span className="flex items-center">{trigger}</span>;

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger className="flex items-center gap-1.5 hover:text-neutral-100">
				{trigger}
			</PopoverTrigger>
			<PopoverContent className="w-96 p-0" align="start">
				<div className="border-b border-neutral-700 px-4 py-2">
					<h3 className="text-sm font-semibold">Parse Failures ({count})</h3>
				</div>
				<ScrollArea className="max-h-64">
					<div className="space-y-1 p-2">
						{failures.map((f) => (
							<div key={f.path} className="rounded px-2 py-1.5 text-xs hover:bg-neutral-800/50">
								<span className="font-mono text-neutral-200">{f.path}</span>
								<p className="mt-0.5 text-neutral-400">{f.reason}</p>
							</div>
						))}
					</div>
				</ScrollArea>
			</PopoverContent>
		</Popover>
	);
}

/** Popover showing unsupported constructs grouped by type. */
function UnsupportedConstructsPopover({
	constructs,
}: {
	constructs: readonly UnsupportedConstruct[];
}): React.JSX.Element {
	const [open, setOpen] = useState(false);
	const count = constructs.length;

	const trigger = (
		<span className={count > 0 ? "text-amber-400" : "text-neutral-400"}>
			<Puzzle className="mr-1.5 inline h-3.5 w-3.5 text-neutral-400" />
			{count} unsupported {count === 1 ? "construct" : "constructs"}
		</span>
	);

	if (count === 0) return <span className="flex items-center">{trigger}</span>;

	// Group by construct type
	const grouped = new Map<UnsupportedConstructType, UnsupportedConstruct[]>();
	for (const c of constructs) {
		const existing = grouped.get(c.constructType) ?? [];
		existing.push(c);
		grouped.set(c.constructType, existing);
	}

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger className="flex items-center gap-1.5 hover:text-neutral-100">
				{trigger}
			</PopoverTrigger>
			<PopoverContent className="w-[28rem] p-0" align="start">
				<div className="border-b border-neutral-700 px-4 py-2">
					<h3 className="text-sm font-semibold">Unsupported Constructs ({count})</h3>
				</div>
				<ScrollArea className="max-h-80">
					<div className="space-y-3 p-3">
						{[...grouped.entries()].map(([type, items]) => (
							<div key={type}>
								<div className="mb-1 flex items-center gap-2">
									<Badge variant="outline" className="text-[10px]">
										{CONSTRUCT_LABELS[type]}
									</Badge>
									<span className="text-xs text-neutral-400">
										{items.length} {items.length === 1 ? "location" : "locations"}
									</span>
								</div>
								<p className="mb-1.5 text-[11px] text-neutral-500">
									{CONSTRUCT_EXPLANATIONS[type]}
								</p>
								<div className="space-y-0.5 pl-2">
									{items.map((item) => (
										<div
											key={`${item.location.path}:${item.location.startLine}`}
											className="text-[11px]"
										>
											<span className="font-mono text-neutral-300">
												{item.location.path}:{item.location.startLine}
											</span>
										</div>
									))}
								</div>
							</div>
						))}
					</div>
				</ScrollArea>
			</PopoverContent>
		</Popover>
	);
}
