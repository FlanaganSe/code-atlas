import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { EdgeFilterBar } from "./components/graph/EdgeFilterBar";
import { GraphCanvas } from "./components/graph/GraphCanvas";
import { CompatibilityPanel } from "./components/panels/CompatibilityPanel";
import { DetailPanel } from "./components/panels/DetailPanel";
import { HealthIndicator } from "./components/panels/HealthIndicator";
import { ProfileBadge } from "./components/panels/ProfileBadge";
import { CommandPalette } from "./components/search/CommandPalette";
import {
	fixtureCompatibilityReport,
	fixtureEdges,
	fixtureGraphHealth,
	fixtureNodes,
	fixtureOverlayEdges,
	fixtureParseFailures,
	fixtureSuppressedEdgeIds,
	fixtureUnresolvedImports,
	fixtureUnsupportedConstructs,
} from "./fixtures/demo-graph";
import { useScan } from "./hooks/use-scan";
import { fitView, getViewport } from "./hooks/viewport-ref";
import { useGraphStore } from "./store/graph-store";
import { useScanStore } from "./store/scan-store";
import type {
	CompatibilityAssessment,
	CompatibilityDetail,
	CompatibilityReport,
	DiscoveryResult,
	SupportStatus,
} from "./types/config";

type AppState =
	| { status: "idle" }
	| { status: "discovering" }
	| { status: "discovered"; result: DiscoveryResult; path: string }
	| { status: "error"; message: string };

export function App(): React.JSX.Element {
	const [state, setState] = useState<AppState>({ status: "idle" });
	const [compatPanelOpen, setCompatPanelOpen] = useState(false);
	const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
	const loadFixture = useGraphStore((s) => s.loadFixture);
	const hasGraph = useGraphStore((s) => s.discoveredNodes.length > 0);
	const expandAll = useGraphStore((s) => s.expandAll);
	const collapseAll = useGraphStore((s) => s.collapseAll);
	const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
	const selectNode = useGraphStore((s) => s.selectNode);
	const deselectNode = useGraphStore((s) => s.deselectNode);
	const expandAncestorsOf = useGraphStore((s) => s.expandAncestorsOf);

	const scanStatus = useScanStore((s) => s.scanStatus);
	const scanProgress = useScanStore((s) => s.progress);
	const scanCompatReport = useScanStore((s) => s.compatibilityReport);
	const scanPath = useScanStore((s) => s.scanPath);
	const { startScan, cancelScan, rescan } = useScan();

	// Use the enriched (post-scan) compatibility report if available, else discovery report
	const discoveryResult = state.status === "discovered" ? state.result : null;
	const activeCompatReport: CompatibilityReport | null =
		scanCompatReport ?? discoveryResult?.compatibility ?? null;

	async function handleOpenDirectory(): Promise<void> {
		setState({ status: "discovering" });
		try {
			const path = await invoke<string | null>("open_directory");
			if (!path) {
				setState({ status: "idle" });
				return;
			}
			const result = await invoke<DiscoveryResult>("discover_workspace", {
				path,
			});
			setState({ status: "discovered", result, path });
		} catch (err) {
			setState({
				status: "error",
				message: err instanceof Error ? err.message : String(err),
			});
		}
	}

	async function handleStartScan(): Promise<void> {
		if (state.status !== "discovered") return;
		await startScan(state.path);
	}

	function handleLoadDemoGraph(): void {
		loadFixture(fixtureNodes, fixtureEdges, fixtureOverlayEdges, new Set(fixtureSuppressedEdgeIds));
		// Populate scan store with demo health/compatibility data
		useScanStore.setState({
			scanStatus: "complete",
			compatibilityReport: fixtureCompatibilityReport,
			graphHealth: fixtureGraphHealth,
			unsupportedConstructs: fixtureUnsupportedConstructs,
			parseFailures: fixtureParseFailures,
			unresolvedImports: fixtureUnresolvedImports,
		});
	}

	async function handleRescan(): Promise<void> {
		const vp = getViewport();
		if (!vp) return;
		await rescan(() => vp);
	}

	// Navigate to a node from search: expand ancestors, select, center
	const handleNavigateToNode = useCallback(
		(nodeId: string) => {
			expandAncestorsOf(nodeId);
			selectNode(nodeId);
			// After layout settles, center on the node
			setTimeout(() => {
				fitView({ nodes: [{ id: nodeId }], duration: 300 });
			}, 500);
		},
		[expandAncestorsOf, selectNode],
	);

	// Keyboard shortcuts
	useEffect(() => {
		function handleKeyDown(e: KeyboardEvent): void {
			// Don't trigger shortcuts when typing in inputs
			const target = e.target as HTMLElement;
			if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) {
				// Allow Escape even in inputs
				if (e.key !== "Escape") return;
			}

			// Cmd+K / Ctrl+K — open command palette
			if ((e.metaKey || e.ctrlKey) && e.key === "k") {
				e.preventDefault();
				setCommandPaletteOpen(true);
				return;
			}

			// Cmd+0 / Ctrl+0 — fit view
			if ((e.metaKey || e.ctrlKey) && e.key === "0") {
				e.preventDefault();
				fitView({ duration: 300 });
				return;
			}

			// Cmd+Shift+E — expand all
			if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "E") {
				e.preventDefault();
				expandAll();
				return;
			}

			// Cmd+Shift+C — collapse all
			if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "C") {
				e.preventDefault();
				collapseAll();
				return;
			}

			// Escape — close palette, deselect, close panel
			if (e.key === "Escape") {
				if (commandPaletteOpen) {
					setCommandPaletteOpen(false);
				} else if (selectedNodeId) {
					deselectNode();
				}
				return;
			}
		}

		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, [commandPaletteOpen, selectedNodeId, expandAll, collapseAll, deselectNode]);

	const isScanning = scanStatus === "scanning";

	return (
		<main className="flex h-screen flex-col bg-neutral-950 text-neutral-100">
			<header className="flex shrink-0 items-center justify-between border-b border-neutral-800 px-6 py-3">
				<div className="flex items-center gap-3">
					<h1 className="text-xl font-semibold">Code Atlas</h1>
					{hasGraph && discoveryResult && (
						<ProfileBadge
							result={discoveryResult}
							onOpenCompatibility={() => setCompatPanelOpen(true)}
						/>
					)}
				</div>
				<div className="flex items-center gap-3">
					{isScanning && scanProgress && (
						<span className="text-xs text-neutral-400">
							Scanning... {scanProgress.scanned}/{scanProgress.total}
						</span>
					)}
					{isScanning && (
						<button
							type="button"
							onClick={() => cancelScan()}
							className="rounded bg-red-900/50 px-3 py-1.5 text-xs font-medium text-red-300 transition-colors hover:bg-red-800/50"
						>
							Cancel Scan
						</button>
					)}
					{hasGraph && !isScanning && scanPath && (
						<button
							type="button"
							onClick={handleRescan}
							className="rounded bg-neutral-800 px-3 py-1.5 text-xs font-medium text-neutral-300 transition-colors hover:bg-neutral-700"
						>
							Rescan
						</button>
					)}
					{hasGraph && (
						<button
							type="button"
							onClick={() => setCommandPaletteOpen(true)}
							className="rounded bg-neutral-800 px-3 py-1.5 text-xs font-medium text-neutral-400 transition-colors hover:bg-neutral-700"
						>
							Search <kbd className="ml-1 rounded bg-neutral-700 px-1 py-0.5 text-[10px]">⌘K</kbd>
						</button>
					)}
					{state.status === "discovered" && !isScanning && (
						<button
							type="button"
							onClick={handleStartScan}
							className="rounded-lg bg-purple-600 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-purple-500"
						>
							Scan
						</button>
					)}
					<button
						type="button"
						onClick={handleLoadDemoGraph}
						className="rounded-lg bg-green-700 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-green-600"
					>
						Load Demo Graph
					</button>
					<button
						type="button"
						onClick={handleOpenDirectory}
						disabled={state.status === "discovering" || isScanning}
						className="rounded-lg bg-blue-600 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
					>
						{state.status === "discovering" ? "Discovering..." : "Open Directory"}
					</button>
				</div>
			</header>

			{/* Health indicator bar */}
			{hasGraph && <HealthIndicator />}

			{/* Edge filter bar */}
			{hasGraph && <EdgeFilterBar />}

			<div className="flex min-h-0 flex-1">
				{hasGraph ? (
					<>
						<div className="min-w-0 flex-1">
							<GraphCanvas />
						</div>
						{/* Detail panel — slides in from right */}
						<div
							className={`overflow-hidden transition-[width] duration-200 ease-in-out ${
								selectedNodeId ? "w-80" : "w-0"
							}`}
						>
							<DetailPanel />
						</div>
					</>
				) : (
					<div className="flex-1 overflow-auto p-6">
						{state.status === "idle" && (
							<IdleView onOpenDirectory={handleOpenDirectory} onLoadDemo={handleLoadDemoGraph} />
						)}
						{state.status === "discovering" && <DiscoveringView />}
						{state.status === "error" && <ErrorView message={state.message} />}
						{state.status === "discovered" && <DiscoveredView result={state.result} />}
					</div>
				)}
			</div>

			{/* Compatibility report panel */}
			{activeCompatReport && (
				<CompatibilityPanel
					report={activeCompatReport}
					open={compatPanelOpen}
					onOpenChange={setCompatPanelOpen}
				/>
			)}

			{/* Command palette */}
			{hasGraph && (
				<CommandPalette
					open={commandPaletteOpen}
					onOpenChange={setCommandPaletteOpen}
					onNavigateToNode={handleNavigateToNode}
				/>
			)}
		</main>
	);
}

// ---------------------------------------------------------------------------
// Pre-scan discovery views (inline — kept from M2)
// ---------------------------------------------------------------------------

function IdleView({
	onOpenDirectory,
	onLoadDemo,
}: {
	onOpenDirectory: () => void;
	onLoadDemo: () => void;
}): React.JSX.Element {
	return (
		<div className="flex flex-col items-center justify-center gap-6 pt-24 text-neutral-400">
			<div className="text-center">
				<h2 className="mb-2 text-2xl font-bold text-neutral-100">Welcome to Code Atlas</h2>
				<p className="max-w-lg text-sm leading-relaxed text-neutral-400">
					Build a profiled, evidence-backed architecture graph from your repository. Discover
					workspace structure, visualize dependencies, and inspect edge provenance — all locally,
					with zero network calls.
				</p>
			</div>
			<div className="flex gap-4">
				<button
					type="button"
					onClick={onOpenDirectory}
					className="rounded-lg bg-blue-600 px-6 py-2.5 text-sm font-medium text-white transition-colors hover:bg-blue-500"
				>
					Open Directory
				</button>
				<button
					type="button"
					onClick={onLoadDemo}
					className="rounded-lg border border-neutral-700 bg-neutral-800 px-6 py-2.5 text-sm font-medium text-neutral-200 transition-colors hover:bg-neutral-700"
				>
					Load Demo
				</button>
			</div>
			<p className="text-xs text-neutral-500">
				Supports Cargo workspaces, pnpm/npm/yarn workspaces, and mixed monorepos.
			</p>
		</div>
	);
}

function DiscoveringView(): React.JSX.Element {
	return (
		<div className="flex flex-col items-center justify-center gap-4 pt-32 text-neutral-400">
			<div className="h-8 w-8 animate-spin rounded-full border-2 border-neutral-600 border-t-blue-500" />
			<p className="text-lg">Discovering workspace...</p>
			<p className="text-sm text-neutral-500">
				This may take a few seconds on first run (cargo metadata).
			</p>
		</div>
	);
}

function ErrorView({ message }: { message: string }): React.JSX.Element {
	return (
		<div className="mx-auto max-w-2xl rounded-lg border border-red-800 bg-red-950/50 p-6">
			<h2 className="mb-2 text-lg font-semibold text-red-400">Discovery Error</h2>
			<p className="font-mono text-sm text-red-300">{message}</p>
		</div>
	);
}

function DiscoveredView({ result }: { result: DiscoveryResult }): React.JSX.Element {
	return (
		<div className="mx-auto grid max-w-5xl gap-6">
			<InlineProfileBadge result={result} />
			<InlineCompatibilityPanel result={result} />
			{result.workspace.packages.length > 0 && <WorkspacePackages result={result} />}
			{result.nonFunctionalConfigSections.length > 0 && (
				<ConfigNotes sections={result.nonFunctionalConfigSections} />
			)}
		</div>
	);
}

function InlineProfileBadge({ result }: { result: DiscoveryResult }): React.JSX.Element {
	const { profile, workspace } = result;

	return (
		<div className="rounded-lg border border-neutral-800 bg-neutral-900 p-5">
			<h2 className="mb-3 text-lg font-semibold">Workspace Profile</h2>
			<div className="grid grid-cols-2 gap-4 text-sm md:grid-cols-4">
				<div>
					<span className="text-neutral-500">Workspace Type</span>
					<p className="mt-1 font-medium">{formatKind(workspace.kind)}</p>
				</div>
				<div>
					<span className="text-neutral-500">Languages</span>
					<p className="mt-1 font-medium">
						{profile.languages.length > 0 ? profile.languages.join(", ") : "None detected"}
					</p>
				</div>
				<div>
					<span className="text-neutral-500">Package Manager</span>
					<p className="mt-1 font-medium">{profile.packageManager ?? "Unknown"}</p>
				</div>
				<div>
					<span className="text-neutral-500">Resolution Mode</span>
					<p className="mt-1 font-medium">{profile.resolutionMode ?? "N/A"}</p>
				</div>
				<div>
					<span className="text-neutral-500">Packages</span>
					<p className="mt-1 font-medium">{workspace.packages.length}</p>
				</div>
				<div>
					<span className="text-neutral-500">Root</span>
					<p className="mt-1 truncate font-mono text-xs" title={workspace.root}>
						{workspace.root}
					</p>
				</div>
			</div>
		</div>
	);
}

function InlineCompatibilityPanel({ result }: { result: DiscoveryResult }): React.JSX.Element {
	const { compatibility } = result;

	return (
		<div className="rounded-lg border border-neutral-800 bg-neutral-900 p-5">
			<div className="mb-4 flex items-center gap-3">
				<h2 className="text-lg font-semibold">Compatibility Report</h2>
				{compatibility.isProvisional && (
					<span className="rounded-full bg-amber-900/50 px-3 py-0.5 text-xs font-medium text-amber-300">
						Provisional
					</span>
				)}
			</div>
			{compatibility.isProvisional && (
				<p className="mb-4 text-sm text-neutral-400">
					This is a structural assessment based on workspace manifests. It will be enriched with
					source-level findings after a full scan.
				</p>
			)}
			{compatibility.assessments.length === 0 ? (
				<p className="text-neutral-500">No language detectors matched this workspace.</p>
			) : (
				<div className="space-y-4">
					{compatibility.assessments.map((assessment) => (
						<AssessmentCard key={assessment.language} assessment={assessment} />
					))}
				</div>
			)}
		</div>
	);
}

function AssessmentCard({
	assessment,
}: {
	assessment: CompatibilityAssessment;
}): React.JSX.Element {
	return (
		<div className="rounded-md border border-neutral-700 bg-neutral-800/50 p-4">
			<div className="mb-3 flex items-center gap-3">
				<span className="text-sm font-semibold uppercase tracking-wide">{assessment.language}</span>
				<StatusBadge status={assessment.status} />
			</div>
			<div className="space-y-2">
				{assessment.details.map((detail) => (
					<DetailRow key={detail.feature} detail={detail} />
				))}
			</div>
		</div>
	);
}

function DetailRow({ detail }: { detail: CompatibilityDetail }): React.JSX.Element {
	return (
		<div className="flex items-start gap-3 text-sm">
			<StatusDot status={detail.status} />
			<div>
				<span className="font-medium">{detail.feature}</span>
				<p className="mt-0.5 text-neutral-400">{detail.explanation}</p>
			</div>
		</div>
	);
}

function StatusBadge({ status }: { status: SupportStatus }): React.JSX.Element {
	const styles = {
		supported: "bg-green-900/50 text-green-300",
		partial: "bg-amber-900/50 text-amber-300",
		unsupported: "bg-red-900/50 text-red-300",
	};
	const labels = {
		supported: "Supported",
		partial: "Partial",
		unsupported: "Unsupported",
	};

	return (
		<span className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${styles[status]}`}>
			{labels[status]}
		</span>
	);
}

function StatusDot({ status }: { status: SupportStatus }): React.JSX.Element {
	const colors = {
		supported: "bg-green-500",
		partial: "bg-amber-500",
		unsupported: "bg-red-500",
	};

	return <span className={`mt-1.5 block h-2 w-2 shrink-0 rounded-full ${colors[status]}`} />;
}

function WorkspacePackages({ result }: { result: DiscoveryResult }): React.JSX.Element {
	return (
		<div className="rounded-lg border border-neutral-800 bg-neutral-900 p-5">
			<h2 className="mb-3 text-lg font-semibold">
				Workspace Packages ({result.workspace.packages.length})
			</h2>
			<div className="space-y-1">
				{result.workspace.packages.map((pkg) => (
					<div
						key={`${pkg.language}:${pkg.relativePath}`}
						className="flex items-center gap-3 rounded px-3 py-2 text-sm hover:bg-neutral-800/50"
					>
						<span className="rounded bg-neutral-700 px-2 py-0.5 font-mono text-xs uppercase">
							{pkg.language}
						</span>
						<span className="font-medium">{pkg.name}</span>
						<span className="text-neutral-500">{pkg.relativePath}</span>
					</div>
				))}
			</div>
		</div>
	);
}

function ConfigNotes({ sections }: { sections: readonly string[] }): React.JSX.Element {
	return (
		<div className="rounded-lg border border-neutral-800 bg-neutral-900 p-5">
			<h2 className="mb-3 text-lg font-semibold">Config Notes</h2>
			<p className="mb-2 text-sm text-neutral-400">
				These .codeatlas.yaml sections are recognized but not yet functional in the POC:
			</p>
			<ul className="space-y-1 text-sm text-neutral-500">
				{sections.map((section) => (
					<li key={section}>- {section}</li>
				))}
			</ul>
		</div>
	);
}

function formatKind(kind: string): string {
	const labels: Record<string, string> = {
		cargo: "Cargo Workspace",
		pnpm: "pnpm Workspace",
		npmYarn: "npm/Yarn Workspace",
		mixed: "Mixed (Cargo + JS)",
		single: "Single Package",
	};
	return labels[kind] ?? kind;
}
