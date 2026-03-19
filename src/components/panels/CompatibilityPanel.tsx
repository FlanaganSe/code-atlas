/**
 * Compatibility Report Panel — per-language assessment cards.
 *
 * Shows support status, feature-by-feature breakdown, and provisional→final
 * transition. Accessible from the profile badge or health indicator.
 *
 * Data source: CompatibilityReport from scanStore or DiscoveryResult.
 */

import { ChevronDown, ChevronRight } from "lucide-react";
import { memo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from "@/components/ui/sheet";
import type {
	CompatibilityAssessment,
	CompatibilityDetail,
	CompatibilityReport,
	SupportStatus,
} from "@/types/config";

interface CompatibilityPanelProps {
	report: CompatibilityReport;
	open: boolean;
	onOpenChange: (open: boolean) => void;
}

export const CompatibilityPanel = memo(function CompatibilityPanel({
	report,
	open,
	onOpenChange,
}: CompatibilityPanelProps): React.JSX.Element {
	return (
		<Sheet open={open} onOpenChange={onOpenChange}>
			<SheetContent className="w-[400px] overflow-hidden border-neutral-800 bg-neutral-900 p-0 sm:max-w-[400px]">
				<SheetHeader className="border-b border-neutral-800 px-4 py-3">
					<SheetTitle className="text-neutral-100">Compatibility Report</SheetTitle>
					<SheetDescription className="text-neutral-400">
						{report.isProvisional
							? "Provisional — structural assessment only. Run a scan for source-level findings."
							: "Final — includes source-level findings from scan."}
					</SheetDescription>
					{report.isProvisional && (
						<Badge variant="outline" className="mt-1 w-fit border-amber-500/30 text-amber-400">
							Provisional
						</Badge>
					)}
				</SheetHeader>
				<ScrollArea className="h-[calc(100vh-8rem)]">
					<div className="space-y-3 p-4">
						{report.assessments.length === 0 ? (
							<p className="text-sm text-neutral-500">
								No language detectors matched this workspace.
							</p>
						) : (
							report.assessments.map((assessment) => (
								<AssessmentCard key={assessment.language} assessment={assessment} />
							))
						)}
					</div>
				</ScrollArea>
			</SheetContent>
		</Sheet>
	);
});

function AssessmentCard({
	assessment,
}: {
	assessment: CompatibilityAssessment;
}): React.JSX.Element {
	const [expanded, setExpanded] = useState(true);

	return (
		<div className="rounded-lg border border-neutral-700 bg-neutral-800/50">
			<button
				type="button"
				className="flex w-full items-center justify-between px-4 py-3"
				onClick={() => setExpanded((e) => !e)}
			>
				<div className="flex items-center gap-3">
					<span className="text-sm font-semibold uppercase tracking-wide text-neutral-200">
						{assessment.language}
					</span>
					<StatusBadge status={assessment.status} />
				</div>
				{expanded ? (
					<ChevronDown className="h-4 w-4 text-neutral-400" />
				) : (
					<ChevronRight className="h-4 w-4 text-neutral-400" />
				)}
			</button>
			{expanded && (
				<div className="border-t border-neutral-700 px-4 py-3">
					<div className="space-y-2">
						{assessment.details.map((detail) => (
							<DetailRow key={detail.feature} detail={detail} />
						))}
					</div>
				</div>
			)}
		</div>
	);
}

function DetailRow({ detail }: { detail: CompatibilityDetail }): React.JSX.Element {
	return (
		<div className="flex items-start gap-3 text-sm">
			<StatusDot status={detail.status} />
			<div>
				<span className="font-medium text-neutral-200">{detail.feature}</span>
				<p className="mt-0.5 text-xs text-neutral-400">{detail.explanation}</p>
			</div>
		</div>
	);
}

function StatusBadge({ status }: { status: SupportStatus }): React.JSX.Element {
	const styles: Record<SupportStatus, string> = {
		supported: "border-green-500/30 text-green-400",
		partial: "border-amber-500/30 text-amber-400",
		unsupported: "border-red-500/30 text-red-400",
	};
	const labels: Record<SupportStatus, string> = {
		supported: "Supported",
		partial: "Partial",
		unsupported: "Unsupported",
	};

	return (
		<Badge variant="outline" className={`text-[10px] ${styles[status]}`}>
			{labels[status]}
		</Badge>
	);
}

function StatusDot({ status }: { status: SupportStatus }): React.JSX.Element {
	const colors: Record<SupportStatus, string> = {
		supported: "bg-green-500",
		partial: "bg-amber-500",
		unsupported: "bg-red-500",
	};

	return <span className={`mt-1.5 block h-2 w-2 shrink-0 rounded-full ${colors[status]}`} />;
}

export { StatusBadge };
