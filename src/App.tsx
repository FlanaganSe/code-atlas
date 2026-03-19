import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import type {
	CompatibilityAssessment,
	CompatibilityDetail,
	DiscoveryResult,
	SupportStatus,
} from "./types/config";

type AppState =
	| { status: "idle" }
	| { status: "discovering" }
	| { status: "discovered"; result: DiscoveryResult }
	| { status: "error"; message: string };

export function App(): React.JSX.Element {
	const [state, setState] = useState<AppState>({ status: "idle" });

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
			setState({ status: "discovered", result });
		} catch (err) {
			setState({
				status: "error",
				message: err instanceof Error ? err.message : String(err),
			});
		}
	}

	return (
		<main className="min-h-screen bg-neutral-950 p-6 text-neutral-100">
			<header className="mb-8 flex items-center justify-between">
				<h1 className="text-2xl font-semibold">Code Atlas</h1>
				<button
					type="button"
					onClick={handleOpenDirectory}
					disabled={state.status === "discovering"}
					className="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white transition-colors hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
				>
					{state.status === "discovering" ? "Discovering..." : "Open Directory"}
				</button>
			</header>

			{state.status === "idle" && <IdleView />}
			{state.status === "discovering" && <DiscoveringView />}
			{state.status === "error" && <ErrorView message={state.message} />}
			{state.status === "discovered" && <DiscoveredView result={state.result} />}
		</main>
	);
}

function IdleView(): React.JSX.Element {
	return (
		<div className="flex flex-col items-center justify-center gap-4 pt-32 text-neutral-400">
			<p className="text-lg">Open a directory to discover its workspace structure.</p>
			<p className="text-sm text-neutral-500">
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
			<ProfileBadge result={result} />
			<CompatibilityReportPanel result={result} />
			{result.workspace.packages.length > 0 && <WorkspacePackages result={result} />}
			{result.nonFunctionalConfigSections.length > 0 && (
				<ConfigNotes sections={result.nonFunctionalConfigSections} />
			)}
		</div>
	);
}

function ProfileBadge({ result }: { result: DiscoveryResult }): React.JSX.Element {
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

function CompatibilityReportPanel({ result }: { result: DiscoveryResult }): React.JSX.Element {
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
