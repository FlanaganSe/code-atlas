/**
 * Hardcoded graph fixture for M3 — exercises all rendering features.
 *
 * Represents a multi-package monorepo with:
 * - 4 packages: @app/web, @lib/core, @lib/utils, @lib/config
 * - Modules within packages (2-3 each)
 * - Files within modules (2-4 each)
 * - Mixed edge categories (value, dev, type_only, normal)
 * - One manual edge (overlay)
 * - One suppressed edge (overlay)
 * - Unsupported construct markers on some nodes
 */

import type { AppEdge, AppNode } from "@/store/graph-projection";
import { keyToId } from "@/store/graph-projection";
import type { EdgeCategory } from "@/types/graph";

// ---------------------------------------------------------------------------
// Helper to create MaterializedKey
// ---------------------------------------------------------------------------

interface MK {
	readonly language: "typescript";
	readonly entityKind: "package" | "module" | "file";
	readonly relativePath: string;
}

function mk(entityKind: "package" | "module" | "file", relativePath: string): MK {
	return { language: "typescript", entityKind, relativePath };
}

// ---------------------------------------------------------------------------
// Deterministic edge ID
// ---------------------------------------------------------------------------

function edgeId(sourceId: string, targetId: string, category: EdgeCategory): string {
	return `edge:${sourceId}→${targetId}:${category}`;
}

// ---------------------------------------------------------------------------
// Node factory
// ---------------------------------------------------------------------------

function packageNode(path: string, label: string, unsupported = 0): AppNode {
	const key = mk("package", path);
	return {
		id: keyToId(key),
		type: "package",
		position: { x: 0, y: 0 },
		data: {
			label,
			kind: "package",
			language: "typescript",
			materializedKey: key,
			parentKey: null,
			isExpanded: false,
			childCount: 0,
			unsupportedConstructs: unsupported,
		},
	};
}

function moduleNode(path: string, label: string, parentPath: string, unsupported = 0): AppNode {
	const key = mk("module", path);
	const parent = mk("package", parentPath);
	return {
		id: keyToId(key),
		type: "module",
		position: { x: 0, y: 0 },
		data: {
			label,
			kind: "module",
			language: "typescript",
			materializedKey: key,
			parentKey: parent,
			isExpanded: false,
			childCount: 0,
			unsupportedConstructs: unsupported,
		},
		parentId: keyToId(parent),
	};
}

function fileNode(
	path: string,
	label: string,
	parentPath: string,
	parentKind: "package" | "module",
	unsupported = 0,
): AppNode {
	const key = mk("file", path);
	const parent = mk(parentKind, parentPath);
	return {
		id: keyToId(key),
		type: "file",
		position: { x: 0, y: 0 },
		data: {
			label,
			kind: "file",
			language: "typescript",
			materializedKey: key,
			parentKey: parent,
			isExpanded: false,
			childCount: 0,
			unsupportedConstructs: unsupported,
		},
		parentId: keyToId(parent),
	};
}

// ---------------------------------------------------------------------------
// Edge factory
// ---------------------------------------------------------------------------

function depEdge(
	sourceId: string,
	targetId: string,
	category: EdgeCategory,
	confidence = "syntactic",
): AppEdge {
	const id = edgeId(sourceId, targetId, category);
	return {
		id,
		source: sourceId,
		target: targetId,
		type: "dependency",
		data: {
			category,
			isManual: false,
			isSuppressed: false,
			isBundled: false,
			bundledEdgeIds: [],
			bundledCount: 0,
			confidence,
			edgeId: id,
		},
	};
}

// ---------------------------------------------------------------------------
// Nodes
// ---------------------------------------------------------------------------

// Packages
const pkgWeb = packageNode("packages/web", "@app/web");
const pkgCore = packageNode("packages/core", "@lib/core");
const pkgUtils = packageNode("packages/utils", "@lib/utils", 1); // unsupported: dynamic import
const pkgConfig = packageNode("packages/config", "@lib/config");

// Modules under @app/web
const modWebPages = moduleNode("packages/web/src/pages", "pages", "packages/web");
const modWebComponents = moduleNode("packages/web/src/components", "components", "packages/web");
const modWebHooks = moduleNode("packages/web/src/hooks", "hooks", "packages/web");

// Modules under @lib/core
const modCoreModels = moduleNode("packages/core/src/models", "models", "packages/core");
const modCoreServices = moduleNode("packages/core/src/services", "services", "packages/core");

// Modules under @lib/utils
const modUtilsHelpers = moduleNode("packages/utils/src/helpers", "helpers", "packages/utils");
const modUtilsFormatters = moduleNode(
	"packages/utils/src/formatters",
	"formatters",
	"packages/utils",
);

// Modules under @lib/config
const modConfigSchema = moduleNode("packages/config/src/schema", "schema", "packages/config");

// Files under pages
const fileHome = fileNode(
	"packages/web/src/pages/Home.tsx",
	"Home.tsx",
	"packages/web/src/pages",
	"module",
);
const fileDashboard = fileNode(
	"packages/web/src/pages/Dashboard.tsx",
	"Dashboard.tsx",
	"packages/web/src/pages",
	"module",
);
const fileSettings = fileNode(
	"packages/web/src/pages/Settings.tsx",
	"Settings.tsx",
	"packages/web/src/pages",
	"module",
);

// Files under components
const fileButton = fileNode(
	"packages/web/src/components/Button.tsx",
	"Button.tsx",
	"packages/web/src/components",
	"module",
);
const fileLayout = fileNode(
	"packages/web/src/components/Layout.tsx",
	"Layout.tsx",
	"packages/web/src/components",
	"module",
);

// Files under hooks
const fileUseAuth = fileNode(
	"packages/web/src/hooks/useAuth.ts",
	"useAuth.ts",
	"packages/web/src/hooks",
	"module",
);

// Files under models
const fileUser = fileNode(
	"packages/core/src/models/User.ts",
	"User.ts",
	"packages/core/src/models",
	"module",
);
const fileProject = fileNode(
	"packages/core/src/models/Project.ts",
	"Project.ts",
	"packages/core/src/models",
	"module",
);

// Files under services
const fileAuthService = fileNode(
	"packages/core/src/services/AuthService.ts",
	"AuthService.ts",
	"packages/core/src/services",
	"module",
);
const fileApiClient = fileNode(
	"packages/core/src/services/ApiClient.ts",
	"ApiClient.ts",
	"packages/core/src/services",
	"module",
	1, // unsupported: dynamic import in ApiClient
);

// Files under helpers
const fileStringUtils = fileNode(
	"packages/utils/src/helpers/stringUtils.ts",
	"stringUtils.ts",
	"packages/utils/src/helpers",
	"module",
);
const fileDateUtils = fileNode(
	"packages/utils/src/helpers/dateUtils.ts",
	"dateUtils.ts",
	"packages/utils/src/helpers",
	"module",
);
const fileArrayUtils = fileNode(
	"packages/utils/src/helpers/arrayUtils.ts",
	"arrayUtils.ts",
	"packages/utils/src/helpers",
	"module",
);

// Files under formatters
const fileCurrency = fileNode(
	"packages/utils/src/formatters/currency.ts",
	"currency.ts",
	"packages/utils/src/formatters",
	"module",
);
const fileNumber = fileNode(
	"packages/utils/src/formatters/number.ts",
	"number.ts",
	"packages/utils/src/formatters",
	"module",
);

// Files under schema
const fileConfigSchema = fileNode(
	"packages/config/src/schema/config.ts",
	"config.ts",
	"packages/config/src/schema",
	"module",
);
const fileValidation = fileNode(
	"packages/config/src/schema/validation.ts",
	"validation.ts",
	"packages/config/src/schema",
	"module",
	1, // unsupported: exports condition
);

// Collect all nodes
export const fixtureNodes: AppNode[] = [
	// Packages first
	pkgWeb,
	pkgCore,
	pkgUtils,
	pkgConfig,
	// Modules
	modWebPages,
	modWebComponents,
	modWebHooks,
	modCoreModels,
	modCoreServices,
	modUtilsHelpers,
	modUtilsFormatters,
	modConfigSchema,
	// Files
	fileHome,
	fileDashboard,
	fileSettings,
	fileButton,
	fileLayout,
	fileUseAuth,
	fileUser,
	fileProject,
	fileAuthService,
	fileApiClient,
	fileStringUtils,
	fileDateUtils,
	fileArrayUtils,
	fileCurrency,
	fileNumber,
	fileConfigSchema,
	fileValidation,
];

// ---------------------------------------------------------------------------
// Edges
// ---------------------------------------------------------------------------

// IDs for readability
const homeId = fileHome.id;
const dashId = fileDashboard.id;
const settingsId = fileSettings.id;
const buttonId = fileButton.id;
const layoutId = fileLayout.id;
const useAuthId = fileUseAuth.id;
const userId = fileUser.id;
const projectId = fileProject.id;
const authSvcId = fileAuthService.id;
const apiClientId = fileApiClient.id;
const stringUtilsId = fileStringUtils.id;
const dateUtilsId = fileDateUtils.id;
const currencyId = fileCurrency.id;
const configSchemaId = fileConfigSchema.id;
const validationId = fileValidation.id;

export const fixtureEdges: AppEdge[] = [
	// Pages → components (value imports)
	depEdge(homeId, buttonId, "value"),
	depEdge(homeId, layoutId, "value"),
	depEdge(dashId, layoutId, "value"),
	depEdge(settingsId, buttonId, "value"),

	// Pages → hooks (value imports)
	depEdge(homeId, useAuthId, "value"),
	depEdge(settingsId, useAuthId, "value"),

	// Pages → core models (type-only imports)
	depEdge(homeId, userId, "typeOnly"),
	depEdge(dashId, projectId, "typeOnly"),
	depEdge(settingsId, userId, "typeOnly"),

	// Hooks → core services (value imports)
	depEdge(useAuthId, authSvcId, "value"),

	// Core services → core models (value imports)
	depEdge(authSvcId, userId, "value"),
	depEdge(apiClientId, projectId, "value"),

	// Core services → utils (value imports)
	depEdge(authSvcId, stringUtilsId, "value"),
	depEdge(apiClientId, dateUtilsId, "value"),

	// Utils cross-module references
	depEdge(currencyId, stringUtilsId, "value"),

	// Config → utils (dev dependency)
	depEdge(validationId, stringUtilsId, "dev"),

	// Layout → config (value import)
	depEdge(layoutId, configSchemaId, "value"),

	// Cross-package type-only
	depEdge(buttonId, userId, "typeOnly"),

	// Dev-only: test utils
	depEdge(dashId, dateUtilsId, "dev"),
];

// ---------------------------------------------------------------------------
// Overlay edges (manual + suppressed)
// ---------------------------------------------------------------------------

// Manual edge: apiClient has an undiscoverable dependency on config
const manualEdgeId = "manual:apiClient→config";
export const fixtureOverlayEdges: AppEdge[] = [
	{
		id: manualEdgeId,
		source: apiClientId,
		target: configSchemaId,
		type: "dependency",
		data: {
			category: "manual",
			isManual: true,
			isSuppressed: false,
			isBundled: false,
			bundledEdgeIds: [],
			bundledCount: 0,
			confidence: "structural",
			edgeId: manualEdgeId,
		},
	},
];

// Suppressed edge: the currency → stringUtils dependency is intentionally suppressed
export const fixtureSuppressedEdgeIds: ReadonlySet<string> = new Set([
	edgeId(currencyId, stringUtilsId, "value"),
]);
