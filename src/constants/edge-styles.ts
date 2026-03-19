/**
 * Shared edge styling constants — single source of truth.
 *
 * Okabe-Ito palette (colorblind-safe) for edge categories.
 */

import type { Confidence, EdgeCategory, EdgeKind } from "@/types/graph";

export const CATEGORY_COLORS: Record<EdgeCategory, string> = {
	value: "#0072B2",
	typeOnly: "#56B4E9",
	dev: "#E69F00",
	build: "#F0E442",
	normal: "#009E73",
	manual: "#CC79A7",
	test: "#D55E00",
	peer: "#999999",
};

export const CATEGORY_LABELS: Record<EdgeCategory, string> = {
	value: "Value",
	typeOnly: "Type-only",
	dev: "Dev",
	build: "Build",
	normal: "Normal",
	manual: "Manual",
	test: "Test",
	peer: "Peer",
};

export const EDGE_DASH: Record<string, string | undefined> = {
	value: undefined, // solid
	typeOnly: "5,5",
	dev: "2,2",
	build: undefined,
	normal: undefined,
	manual: undefined, // uses stroke-width instead
	test: "8,4",
	peer: "4,4",
};

export const SUPPRESSED_COLOR = "#999999";
export const SUPPRESSED_DASH = "10,5";

export const KIND_LABELS: Record<EdgeKind, string> = {
	imports: "Imports",
	reExports: "Re-exports",
	contains: "Contains",
	dependsOn: "Depends on",
	manual: "Manual",
};

export const CONFIDENCE_DESCRIPTIONS: Record<Confidence, string> = {
	structural: "From manifests/config",
	syntactic: "From source code parsing",
	resolverAware: "Validated against filesystem",
	semantic: "Type-system aware",
	runtime: "Observed at execution",
};
