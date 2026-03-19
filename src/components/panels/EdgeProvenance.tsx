/**
 * Edge Provenance Popover — shows full provenance data on edge click.
 *
 * Displays: kind, category, confidence, source location, resolution method,
 * overlay status. For bundled edges, shows count and category breakdown.
 *
 * Data source: AppEdgeData from the graph store (carries all provenance fields).
 */

import { memo, useCallback, useEffect, useRef } from "react";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
	CATEGORY_COLORS,
	CATEGORY_LABELS,
	CONFIDENCE_DESCRIPTIONS,
	KIND_LABELS,
} from "@/constants/edge-styles";
import type { AppEdgeData } from "@/store/graph-projection";

interface EdgeProvenancePopoverProps {
	/** Screen position to anchor the popover. */
	position: { x: number; y: number } | null;
	/** The clicked edge data. */
	edgeData: AppEdgeData | null;
	/** Called to dismiss the popover. */
	onClose: () => void;
}

export const EdgeProvenancePopover = memo(function EdgeProvenancePopover({
	position,
	edgeData,
	onClose,
}: EdgeProvenancePopoverProps): React.JSX.Element | null {
	const ref = useRef<HTMLDivElement>(null);

	const handleClickOutside = useCallback(
		(e: MouseEvent) => {
			if (ref.current && !ref.current.contains(e.target as Node)) {
				onClose();
			}
		},
		[onClose],
	);

	useEffect(() => {
		if (position) {
			document.addEventListener("mousedown", handleClickOutside);
			return () => document.removeEventListener("mousedown", handleClickOutside);
		}
	}, [position, handleClickOutside]);

	if (!position || !edgeData) return null;

	return (
		<div
			ref={ref}
			className="fixed z-50 w-72 rounded-lg border border-neutral-700 bg-neutral-900 p-3 text-xs shadow-xl"
			style={{ left: position.x + 8, top: position.y + 8 }}
		>
			{edgeData.isBundled ? (
				<BundledEdgeContent edgeData={edgeData} />
			) : (
				<SingleEdgeContent edgeData={edgeData} />
			)}
		</div>
	);
});

function SingleEdgeContent({ edgeData }: { edgeData: AppEdgeData }): React.JSX.Element {
	const confidence = edgeData.confidence;

	return (
		<div className="space-y-2">
			{/* Category with color dot + badges */}
			<div className="flex items-center justify-between">
				<div className="flex items-center gap-2">
					<span
						className="inline-block h-2.5 w-2.5 rounded-full"
						style={{ backgroundColor: CATEGORY_COLORS[edgeData.category] }}
					/>
					<span className="font-medium text-neutral-100">{CATEGORY_LABELS[edgeData.category]}</span>
				</div>
				<div className="flex gap-1">
					{edgeData.isManual && (
						<Badge variant="outline" className="border-pink-400/30 text-[10px] text-pink-400">
							manual
						</Badge>
					)}
					{edgeData.isSuppressed && (
						<Badge variant="outline" className="border-neutral-500/30 text-[10px] text-neutral-400">
							suppressed
						</Badge>
					)}
				</div>
			</div>

			<Separator className="my-1" />

			{/* Kind */}
			<Row label="Kind" value={KIND_LABELS[edgeData.kind]} />

			{/* Confidence */}
			<Row
				label="Confidence"
				value={`${confidence} — ${CONFIDENCE_DESCRIPTIONS[confidence] ?? confidence}`}
			/>

			{/* Source location */}
			{edgeData.sourceLocation && (
				<Row
					label="Location"
					value={`${edgeData.sourceLocation.path}:${edgeData.sourceLocation.startLine}-${edgeData.sourceLocation.endLine}`}
				/>
			)}

			{/* Resolution method */}
			{edgeData.resolutionMethod && <Row label="Resolution" value={edgeData.resolutionMethod} />}

			{/* Overlay status */}
			{edgeData.isManual && <Row label="Source" value="Declared in .codeatlas.yaml" />}
			{edgeData.suppressionReason && (
				<Row label="Suppression reason" value={edgeData.suppressionReason} />
			)}
		</div>
	);
}

function BundledEdgeContent({ edgeData }: { edgeData: AppEdgeData }): React.JSX.Element {
	return (
		<div className="space-y-2">
			<div className="flex items-center gap-2">
				<span className="font-medium text-neutral-100">
					{edgeData.bundledCount} underlying {edgeData.bundledCount === 1 ? "edge" : "edges"}
				</span>
			</div>

			<Separator className="my-1" />

			<Row label="Category" value={CATEGORY_LABELS[edgeData.category]} />
			<Row label="Confidence" value={edgeData.confidence} />

			<p className="mt-1 text-[10px] text-neutral-500">
				Expand the source or target package to see individual edges.
			</p>
		</div>
	);
}

function Row({ label, value }: { label: string; value: string }): React.JSX.Element {
	return (
		<div className="flex justify-between gap-2">
			<span className="shrink-0 text-neutral-500">{label}</span>
			<span className="text-right text-neutral-200">{value}</span>
		</div>
	);
}
