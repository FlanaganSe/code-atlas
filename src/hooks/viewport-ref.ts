/**
 * Module-level ref for React Flow viewport functions.
 *
 * GraphCanvasInner registers the viewport getter/setter here,
 * and parent components can access them without being inside ReactFlowProvider.
 */

type ViewportFns = {
	getViewport: () => { x: number; y: number; zoom: number };
	fitView: (options?: { nodes?: { id: string }[]; duration?: number }) => void;
};

let viewportFns: ViewportFns | null = null;

export function registerViewportFns(fns: ViewportFns): void {
	viewportFns = fns;
}

export function getViewport(): { x: number; y: number; zoom: number } | null {
	return viewportFns ? viewportFns.getViewport() : null;
}

export function fitView(options?: { nodes?: { id: string }[]; duration?: number }): void {
	viewportFns?.fitView(options);
}
