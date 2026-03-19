/**
 * Edge Filter Bar — toggle buttons for edge categories + show suppressed.
 *
 * Toggling a category calls graphStore.setCategoryFilter() → projection re-runs → ELK re-layouts.
 * This bar appears above the graph canvas when a graph is loaded.
 */

import { memo, useCallback } from "react";
import { useGraphStore } from "@/store/graph-store";
import type { EdgeCategory } from "@/types/graph";

const CATEGORIES: { key: EdgeCategory; label: string; color: string }[] = [
	{ key: "value", label: "Value", color: "#0072B2" },
	{ key: "typeOnly", label: "Type-only", color: "#56B4E9" },
	{ key: "dev", label: "Dev", color: "#E69F00" },
	{ key: "build", label: "Build", color: "#F0E442" },
	{ key: "normal", label: "Normal", color: "#009E73" },
	{ key: "manual", label: "Manual", color: "#CC79A7" },
];

export const EdgeFilterBar = memo(function EdgeFilterBar(): React.JSX.Element {
	const categoryFilter = useGraphStore((s) => s.categoryFilter);
	const setCategoryFilter = useGraphStore((s) => s.setCategoryFilter);
	const showSuppressed = useGraphStore((s) => s.showSuppressed);
	const toggleSuppressed = useGraphStore((s) => s.toggleSuppressed);

	const toggleCategory = useCallback(
		(cat: EdgeCategory) => {
			const next = new Set(categoryFilter);
			if (next.has(cat)) {
				next.delete(cat);
			} else {
				next.add(cat);
			}
			setCategoryFilter(next);
		},
		[categoryFilter, setCategoryFilter],
	);

	return (
		<div className="flex items-center gap-2 border-b border-neutral-800 bg-neutral-900/60 px-4 py-1.5">
			<span className="mr-1 text-[10px] font-medium uppercase tracking-wider text-neutral-500">
				Edges
			</span>
			{CATEGORIES.map(({ key, label, color }) => {
				const active = categoryFilter.has(key);
				return (
					<button
						key={key}
						type="button"
						onClick={() => toggleCategory(key)}
						className={`flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium transition-colors ${
							active
								? "bg-neutral-800 text-neutral-200"
								: "text-neutral-500 ring-1 ring-neutral-700 hover:text-neutral-300"
						}`}
					>
						<span
							className="inline-block h-2 w-2 rounded-full"
							style={{
								backgroundColor: color,
								opacity: active ? 1 : 0.3,
							}}
						/>
						{label}
					</button>
				);
			})}

			<div className="mx-1 h-4 w-px bg-neutral-700" />

			<button
				type="button"
				onClick={toggleSuppressed}
				className={`rounded-full px-2.5 py-1 text-[11px] font-medium transition-colors ${
					showSuppressed
						? "bg-amber-900/50 text-amber-300"
						: "text-neutral-500 ring-1 ring-neutral-700 hover:text-neutral-300"
				}`}
			>
				{showSuppressed ? "Suppressed ✓" : "Suppressed"}
			</button>
		</div>
	);
});
