/**
 * Hook to trigger ELK layout on expand/collapse/data changes.
 *
 * Watches `layoutVersion` (incremented by the store on each projection)
 * to avoid a dependency cycle: layout updates projectedNodes positions,
 * which would re-trigger layout if we watched projectedNodes directly.
 */

import { useEffect, useRef } from "react";
import { layoutGraph } from "@/components/graph/layout/elk-layout";
import { useGraphStore } from "@/store/graph-store";

export function useLayout(): void {
	const layoutVersion = useGraphStore((s) => s.layoutVersion);
	const isRunning = useRef(false);

	// biome-ignore lint/correctness/useExhaustiveDependencies: layoutVersion is an intentional trigger to avoid dependency cycle
	useEffect(() => {
		const { projectedNodes, projectedEdges, expandedNodeIds } = useGraphStore.getState();
		if (projectedNodes.length === 0) return;
		if (isRunning.current) return;

		isRunning.current = true;

		layoutGraph(projectedNodes, projectedEdges, expandedNodeIds)
			.then((positioned) => {
				useGraphStore.setState({ projectedNodes: positioned });
			})
			.finally(() => {
				isRunning.current = false;
			});
	}, [layoutVersion]);
}
