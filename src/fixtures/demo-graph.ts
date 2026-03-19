/**
 * Product demo fixture — exercises every interaction pattern.
 *
 * Models a realistic SaaS monorepo with:
 * - 6 packages: @app/web, @app/api, @lib/auth, @lib/database, @lib/shared, @tools/cli
 * - Nested modules (components/ui, components/layout under @app/web)
 * - ~65 total nodes, ~45 edges
 * - All edge categories: value, typeOnly, dev, build, normal, manual, suppressed
 * - Unsupported constructs: dynamic import, cfg gate, exports condition
 * - Unresolved imports: 3 with different reasons
 * - Parse failure: 1 file
 * - Edge provenance data (source_location, resolution_method, confidence)
 * - Matching compatibility report and graph health
 */

import type { AppEdge, AppNode } from "@/store/graph-projection";
import { keyToId } from "@/store/graph-projection";
import type { CompatibilityReport, GraphHealth, UnresolvedImport } from "@/types/config";
import type { Confidence, EdgeCategory, ParseFailure, UnsupportedConstruct } from "@/types/graph";

// ---------------------------------------------------------------------------
// MaterializedKey helpers
// ---------------------------------------------------------------------------

interface MK {
	readonly language: "typescript" | "rust";
	readonly entityKind: "package" | "module" | "file";
	readonly relativePath: string;
}

function mk(
	language: "typescript" | "rust",
	entityKind: "package" | "module" | "file",
	relativePath: string,
): MK {
	return { language, entityKind, relativePath };
}

function edgeId(sourceId: string, targetId: string, category: EdgeCategory): string {
	return `edge:${sourceId}→${targetId}:${category}`;
}

// ---------------------------------------------------------------------------
// Node factories
// ---------------------------------------------------------------------------

function packageNode(
	language: "typescript" | "rust",
	path: string,
	label: string,
	unsupported = 0,
): AppNode {
	const key = mk(language, "package", path);
	return {
		id: keyToId(key),
		type: "package",
		position: { x: 0, y: 0 },
		data: {
			label,
			kind: "package",
			language,
			materializedKey: key,
			parentKey: null,
			isExpanded: false,
			childCount: 0,
			unsupportedConstructs: unsupported,
		},
	};
}

function moduleNode(
	language: "typescript" | "rust",
	path: string,
	label: string,
	parentPath: string,
	parentKind: "package" | "module" = "package",
	unsupported = 0,
): AppNode {
	const key = mk(language, "module", path);
	const parent = mk(language, parentKind, parentPath);
	return {
		id: keyToId(key),
		type: "module",
		position: { x: 0, y: 0 },
		data: {
			label,
			kind: "module",
			language,
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
	language: "typescript" | "rust",
	path: string,
	label: string,
	parentPath: string,
	parentKind: "package" | "module",
	unsupported = 0,
): AppNode {
	const key = mk(language, "file", path);
	const parent = mk(language, parentKind, parentPath);
	return {
		id: keyToId(key),
		type: "file",
		position: { x: 0, y: 0 },
		data: {
			label,
			kind: "file",
			language,
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
// Edge factory with provenance
// ---------------------------------------------------------------------------

function depEdge(
	sourceId: string,
	targetId: string,
	category: EdgeCategory,
	opts: {
		confidence?: Confidence;
		sourceLocation?: { path: string; startLine: number; endLine: number } | null;
		resolutionMethod?: string | null;
		kind?: "imports" | "reExports" | "contains" | "dependsOn" | "manual";
	} = {},
): AppEdge {
	const id = edgeId(sourceId, targetId, category);
	return {
		id,
		source: sourceId,
		target: targetId,
		type: "dependency",
		data: {
			category,
			kind: opts.kind ?? "imports",
			isManual: false,
			isSuppressed: false,
			isBundled: false,
			bundledEdgeIds: [],
			bundledCount: 0,
			confidence: opts.confidence ?? "syntactic",
			edgeId: id,
			sourceLocation: opts.sourceLocation ?? null,
			resolutionMethod: opts.resolutionMethod ?? null,
			suppressionReason: null,
		},
	};
}

// ===========================================================================
// PACKAGES (6)
// ===========================================================================

const pkgWeb = packageNode("typescript", "packages/web", "@app/web");
const pkgApi = packageNode("typescript", "packages/api", "@app/api");
const pkgAuth = packageNode("typescript", "packages/auth", "@lib/auth");
const pkgDatabase = packageNode("rust", "crates/database", "@lib/database", 1);
const pkgShared = packageNode("typescript", "packages/shared", "@lib/shared");
const pkgCli = packageNode("rust", "tools/cli", "@tools/cli");

// ===========================================================================
// MODULES (15) — including 2 nested under @app/web/components
// ===========================================================================

// @app/web modules
const modWebPages = moduleNode("typescript", "packages/web/src/pages", "pages", "packages/web");
const modWebComponents = moduleNode(
	"typescript",
	"packages/web/src/components",
	"components",
	"packages/web",
);
// Nested modules under components (2 levels deep)
const modWebUi = moduleNode(
	"typescript",
	"packages/web/src/components/ui",
	"ui",
	"packages/web/src/components",
	"module",
);
const modWebLayout = moduleNode(
	"typescript",
	"packages/web/src/components/layout",
	"layout",
	"packages/web/src/components",
	"module",
);
const modWebHooks = moduleNode("typescript", "packages/web/src/hooks", "hooks", "packages/web");

// @app/api modules
const modApiRoutes = moduleNode("typescript", "packages/api/src/routes", "routes", "packages/api");
const modApiMiddleware = moduleNode(
	"typescript",
	"packages/api/src/middleware",
	"middleware",
	"packages/api",
);
const modApiServices = moduleNode(
	"typescript",
	"packages/api/src/services",
	"services",
	"packages/api",
);

// @lib/auth modules
const modAuthProviders = moduleNode(
	"typescript",
	"packages/auth/src/providers",
	"providers",
	"packages/auth",
);
const modAuthGuards = moduleNode(
	"typescript",
	"packages/auth/src/guards",
	"guards",
	"packages/auth",
);

// @lib/database modules (Rust)
const modDbModels = moduleNode("rust", "crates/database/src/models", "models", "crates/database");
const modDbPool = moduleNode("rust", "crates/database/src/pool", "pool", "crates/database");

// @lib/shared modules
const modSharedUtils = moduleNode(
	"typescript",
	"packages/shared/src/utils",
	"utils",
	"packages/shared",
);
const modSharedTypes = moduleNode(
	"typescript",
	"packages/shared/src/types",
	"types",
	"packages/shared",
);

// @tools/cli modules (Rust)
const modCliCommands = moduleNode("rust", "tools/cli/src/commands", "commands", "tools/cli");

// ===========================================================================
// FILES (43)
// ===========================================================================

// --- @app/web files ---
const fileApp = fileNode(
	"typescript",
	"packages/web/src/App.tsx",
	"App.tsx",
	"packages/web",
	"package",
);
const fileHome = fileNode(
	"typescript",
	"packages/web/src/pages/Home.tsx",
	"Home.tsx",
	"packages/web/src/pages",
	"module",
);
const fileDashboard = fileNode(
	"typescript",
	"packages/web/src/pages/Dashboard.tsx",
	"Dashboard.tsx",
	"packages/web/src/pages",
	"module",
);
const fileSettings = fileNode(
	"typescript",
	"packages/web/src/pages/Settings.tsx",
	"Settings.tsx",
	"packages/web/src/pages",
	"module",
);
const fileButton = fileNode(
	"typescript",
	"packages/web/src/components/ui/Button.tsx",
	"Button.tsx",
	"packages/web/src/components/ui",
	"module",
);
const fileCard = fileNode(
	"typescript",
	"packages/web/src/components/ui/Card.tsx",
	"Card.tsx",
	"packages/web/src/components/ui",
	"module",
);
const fileInput = fileNode(
	"typescript",
	"packages/web/src/components/ui/Input.tsx",
	"Input.tsx",
	"packages/web/src/components/ui",
	"module",
);
const fileShell = fileNode(
	"typescript",
	"packages/web/src/components/layout/Shell.tsx",
	"Shell.tsx",
	"packages/web/src/components/layout",
	"module",
);
const fileSidebar = fileNode(
	"typescript",
	"packages/web/src/components/layout/Sidebar.tsx",
	"Sidebar.tsx",
	"packages/web/src/components/layout",
	"module",
);
const fileUseAuth = fileNode(
	"typescript",
	"packages/web/src/hooks/useAuth.ts",
	"useAuth.ts",
	"packages/web/src/hooks",
	"module",
);
const fileUseTheme = fileNode(
	"typescript",
	"packages/web/src/hooks/useTheme.ts",
	"useTheme.ts",
	"packages/web/src/hooks",
	"module",
);

// --- @app/api files ---
const fileServer = fileNode(
	"typescript",
	"packages/api/src/server.ts",
	"server.ts",
	"packages/api",
	"package",
);
const fileApiIndex = fileNode(
	"typescript",
	"packages/api/src/index.ts",
	"index.ts",
	"packages/api",
	"package",
);
const fileRouteAuth = fileNode(
	"typescript",
	"packages/api/src/routes/auth.ts",
	"auth.ts",
	"packages/api/src/routes",
	"module",
);
const fileRouteUsers = fileNode(
	"typescript",
	"packages/api/src/routes/users.ts",
	"users.ts",
	"packages/api/src/routes",
	"module",
);
const fileRouteProjects = fileNode(
	"typescript",
	"packages/api/src/routes/projects.ts",
	"projects.ts",
	"packages/api/src/routes",
	"module",
);
const fileCors = fileNode(
	"typescript",
	"packages/api/src/middleware/cors.ts",
	"cors.ts",
	"packages/api/src/middleware",
	"module",
);
const fileRateLimit = fileNode(
	"typescript",
	"packages/api/src/middleware/rateLimit.ts",
	"rateLimit.ts",
	"packages/api/src/middleware",
	"module",
);
const fileCache = fileNode(
	"typescript",
	"packages/api/src/services/cache.ts",
	"cache.ts",
	"packages/api/src/services",
	"module",
);
const fileEmail = fileNode(
	"typescript",
	"packages/api/src/services/email.ts",
	"email.ts",
	"packages/api/src/services",
	"module",
	1,
);

// --- @lib/auth files ---
const fileAuthIndex = fileNode(
	"typescript",
	"packages/auth/src/index.ts",
	"index.ts",
	"packages/auth",
	"package",
);
const fileOAuth = fileNode(
	"typescript",
	"packages/auth/src/providers/oauth.ts",
	"oauth.ts",
	"packages/auth/src/providers",
	"module",
	1,
);
const fileJwt = fileNode(
	"typescript",
	"packages/auth/src/providers/jwt.ts",
	"jwt.ts",
	"packages/auth/src/providers",
	"module",
);
const fileRbac = fileNode(
	"typescript",
	"packages/auth/src/guards/rbac.ts",
	"rbac.ts",
	"packages/auth/src/guards",
	"module",
);
const fileSession = fileNode(
	"typescript",
	"packages/auth/src/guards/session.ts",
	"session.ts",
	"packages/auth/src/guards",
	"module",
);

// --- @lib/database files (Rust) ---
const fileDbLib = fileNode(
	"rust",
	"crates/database/src/lib.rs",
	"lib.rs",
	"crates/database",
	"package",
);
const fileUserModel = fileNode(
	"rust",
	"crates/database/src/models/user.rs",
	"user.rs",
	"crates/database/src/models",
	"module",
);
const fileProjectModel = fileNode(
	"rust",
	"crates/database/src/models/project.rs",
	"project.rs",
	"crates/database/src/models",
	"module",
);
const fileMigration = fileNode(
	"rust",
	"crates/database/src/models/migration.rs",
	"migration.rs",
	"crates/database/src/models",
	"module",
);
const fileConnection = fileNode(
	"rust",
	"crates/database/src/pool/connection.rs",
	"connection.rs",
	"crates/database/src/pool",
	"module",
	1,
);
const fileDbConfig = fileNode(
	"rust",
	"crates/database/src/pool/config.rs",
	"config.rs",
	"crates/database/src/pool",
	"module",
);

// --- @lib/shared files ---
const fileSharedConstants = fileNode(
	"typescript",
	"packages/shared/src/constants.ts",
	"constants.ts",
	"packages/shared",
	"package",
);
const fileStringUtil = fileNode(
	"typescript",
	"packages/shared/src/utils/string.ts",
	"string.ts",
	"packages/shared/src/utils",
	"module",
);
const fileDateUtil = fileNode(
	"typescript",
	"packages/shared/src/utils/date.ts",
	"date.ts",
	"packages/shared/src/utils",
	"module",
);
const fileValidation = fileNode(
	"typescript",
	"packages/shared/src/utils/validation.ts",
	"validation.ts",
	"packages/shared/src/utils",
	"module",
);
const fileCommonTypes = fileNode(
	"typescript",
	"packages/shared/src/types/common.ts",
	"common.ts",
	"packages/shared/src/types",
	"module",
);
const fileErrorTypes = fileNode(
	"typescript",
	"packages/shared/src/types/errors.ts",
	"errors.ts",
	"packages/shared/src/types",
	"module",
);
const fileApiTypes = fileNode(
	"typescript",
	"packages/shared/src/types/api-types.ts",
	"api-types.ts",
	"packages/shared/src/types",
	"module",
	1,
);

// --- @tools/cli files (Rust) ---
const fileCliMain = fileNode("rust", "tools/cli/src/main.rs", "main.rs", "tools/cli", "package");
const fileCmdInit = fileNode(
	"rust",
	"tools/cli/src/commands/init.rs",
	"init.rs",
	"tools/cli/src/commands",
	"module",
);
const fileCmdScan = fileNode(
	"rust",
	"tools/cli/src/commands/scan.rs",
	"scan.rs",
	"tools/cli/src/commands",
	"module",
);
const fileCmdReport = fileNode(
	"rust",
	"tools/cli/src/commands/report.rs",
	"report.rs",
	"tools/cli/src/commands",
	"module",
);
const fileFormatBroken = fileNode(
	"rust",
	"tools/cli/src/commands/format-broken.rs",
	"format-broken.rs",
	"tools/cli/src/commands",
	"module",
);

// ===========================================================================
// All nodes (64 total: 6 packages + 15 modules + 43 files)
// ===========================================================================

export const fixtureNodes: AppNode[] = [
	// Packages (6)
	pkgWeb,
	pkgApi,
	pkgAuth,
	pkgDatabase,
	pkgShared,
	pkgCli,
	// Modules (16)
	modWebPages,
	modWebComponents,
	modWebUi,
	modWebLayout,
	modWebHooks,
	modApiRoutes,
	modApiMiddleware,
	modApiServices,
	modAuthProviders,
	modAuthGuards,
	modDbModels,
	modDbPool,
	modSharedUtils,
	modSharedTypes,
	modCliCommands,
	// Files (43)
	fileApp,
	fileHome,
	fileDashboard,
	fileSettings,
	fileButton,
	fileCard,
	fileInput,
	fileShell,
	fileSidebar,
	fileUseAuth,
	fileUseTheme,
	fileServer,
	fileApiIndex,
	fileRouteAuth,
	fileRouteUsers,
	fileRouteProjects,
	fileCors,
	fileRateLimit,
	fileCache,
	fileEmail,
	fileAuthIndex,
	fileOAuth,
	fileJwt,
	fileRbac,
	fileSession,
	fileDbLib,
	fileUserModel,
	fileProjectModel,
	fileMigration,
	fileConnection,
	fileDbConfig,
	fileSharedConstants,
	fileStringUtil,
	fileDateUtil,
	fileValidation,
	fileCommonTypes,
	fileErrorTypes,
	fileApiTypes,
	fileCliMain,
	fileCmdInit,
	fileCmdScan,
	fileCmdReport,
	fileFormatBroken,
];

// ===========================================================================
// EDGES (51 discovered + 1 manual + 1 suppressed)
// ===========================================================================

// Node IDs for readability
const homeId = fileHome.id;
const dashId = fileDashboard.id;
const settingsId = fileSettings.id;
const appId = fileApp.id;
const buttonId = fileButton.id;
const cardId = fileCard.id;
const inputId = fileInput.id;
const shellId = fileShell.id;
const sidebarId = fileSidebar.id;
const useAuthId = fileUseAuth.id;
const useThemeId = fileUseTheme.id;
const serverId = fileServer.id;
const apiIndexId = fileApiIndex.id;
const routeAuthId = fileRouteAuth.id;
const routeUsersId = fileRouteUsers.id;
const routeProjectsId = fileRouteProjects.id;
const corsId = fileCors.id;
const rateLimitId = fileRateLimit.id;
const cacheId = fileCache.id;
const emailId = fileEmail.id;
const authIndexId = fileAuthIndex.id;
const oauthId = fileOAuth.id;
const jwtId = fileJwt.id;
const rbacId = fileRbac.id;
const sessionId = fileSession.id;
const dbLibId = fileDbLib.id;
const userModelId = fileUserModel.id;
const projectModelId = fileProjectModel.id;
const migrationId = fileMigration.id;
const connectionId = fileConnection.id;
const dbConfigId = fileDbConfig.id;
const constantsId = fileSharedConstants.id;
const stringUtilId = fileStringUtil.id;
const dateUtilId = fileDateUtil.id;
const validationId = fileValidation.id;
const commonTypesId = fileCommonTypes.id;
const errorTypesId = fileErrorTypes.id;
const apiTypesId = fileApiTypes.id;
const cliMainId = fileCliMain.id;
const cmdInitId = fileCmdInit.id;
const cmdScanId = fileCmdScan.id;
const cmdReportId = fileCmdReport.id;

export const fixtureEdges: AppEdge[] = [
	// --- @app/web internal ---
	// App.tsx → pages
	depEdge(appId, homeId, "value", {
		confidence: "syntactic",
		sourceLocation: { path: "packages/web/src/App.tsx", startLine: 3, endLine: 3 },
		resolutionMethod: "tree-sitter import",
	}),
	depEdge(appId, dashId, "value", {
		sourceLocation: { path: "packages/web/src/App.tsx", startLine: 4, endLine: 4 },
		resolutionMethod: "tree-sitter import",
	}),
	depEdge(appId, settingsId, "value", {
		sourceLocation: { path: "packages/web/src/App.tsx", startLine: 5, endLine: 5 },
		resolutionMethod: "tree-sitter import",
	}),
	// App.tsx → layout
	depEdge(appId, shellId, "value", {
		sourceLocation: { path: "packages/web/src/App.tsx", startLine: 6, endLine: 6 },
		resolutionMethod: "tree-sitter import",
	}),
	// Pages → UI components
	depEdge(homeId, buttonId, "value", {
		sourceLocation: { path: "packages/web/src/pages/Home.tsx", startLine: 2, endLine: 2 },
		resolutionMethod: "tree-sitter import",
	}),
	depEdge(homeId, cardId, "value", {
		sourceLocation: { path: "packages/web/src/pages/Home.tsx", startLine: 3, endLine: 3 },
		resolutionMethod: "tree-sitter import",
	}),
	depEdge(dashId, cardId, "value", {
		sourceLocation: { path: "packages/web/src/pages/Dashboard.tsx", startLine: 2, endLine: 2 },
		resolutionMethod: "tree-sitter import",
	}),
	depEdge(settingsId, inputId, "value", {
		sourceLocation: { path: "packages/web/src/pages/Settings.tsx", startLine: 2, endLine: 2 },
		resolutionMethod: "tree-sitter import",
	}),
	depEdge(settingsId, buttonId, "value", {
		sourceLocation: { path: "packages/web/src/pages/Settings.tsx", startLine: 3, endLine: 3 },
		resolutionMethod: "tree-sitter import",
	}),
	// Layout → UI components
	depEdge(shellId, sidebarId, "value", {
		sourceLocation: {
			path: "packages/web/src/components/layout/Shell.tsx",
			startLine: 1,
			endLine: 1,
		},
		resolutionMethod: "tree-sitter import",
	}),
	// Pages → hooks
	depEdge(homeId, useAuthId, "value", {
		sourceLocation: { path: "packages/web/src/pages/Home.tsx", startLine: 4, endLine: 4 },
		resolutionMethod: "tree-sitter import",
	}),
	depEdge(settingsId, useAuthId, "value", {
		sourceLocation: { path: "packages/web/src/pages/Settings.tsx", startLine: 4, endLine: 4 },
		resolutionMethod: "tree-sitter import",
	}),
	depEdge(shellId, useThemeId, "value", {
		sourceLocation: {
			path: "packages/web/src/components/layout/Shell.tsx",
			startLine: 2,
			endLine: 2,
		},
		resolutionMethod: "tree-sitter import",
	}),

	// --- @app/web → @lib/shared (type-only) ---
	depEdge(homeId, commonTypesId, "typeOnly", {
		confidence: "syntactic",
		sourceLocation: { path: "packages/web/src/pages/Home.tsx", startLine: 1, endLine: 1 },
		resolutionMethod: "tree-sitter import type",
	}),
	depEdge(dashId, apiTypesId, "typeOnly", {
		sourceLocation: { path: "packages/web/src/pages/Dashboard.tsx", startLine: 1, endLine: 1 },
		resolutionMethod: "tree-sitter import type",
	}),
	depEdge(useAuthId, errorTypesId, "typeOnly", {
		sourceLocation: { path: "packages/web/src/hooks/useAuth.ts", startLine: 1, endLine: 1 },
		resolutionMethod: "tree-sitter import type",
	}),

	// --- @app/web → @lib/auth (value) ---
	depEdge(useAuthId, authIndexId, "value", {
		sourceLocation: { path: "packages/web/src/hooks/useAuth.ts", startLine: 2, endLine: 2 },
		resolutionMethod: "workspace package resolution",
	}),

	// --- @app/web → @lib/shared (value) ---
	depEdge(appId, constantsId, "value", {
		sourceLocation: { path: "packages/web/src/App.tsx", startLine: 8, endLine: 8 },
		resolutionMethod: "workspace package resolution",
	}),
	depEdge(settingsId, validationId, "value", {
		sourceLocation: { path: "packages/web/src/pages/Settings.tsx", startLine: 6, endLine: 6 },
		resolutionMethod: "workspace package resolution",
	}),

	// --- @app/api internal ---
	depEdge(serverId, routeAuthId, "value", {
		sourceLocation: { path: "packages/api/src/server.ts", startLine: 3, endLine: 3 },
		resolutionMethod: "tree-sitter import",
	}),
	depEdge(serverId, routeUsersId, "value"),
	depEdge(serverId, routeProjectsId, "value"),
	depEdge(serverId, corsId, "value"),
	depEdge(serverId, rateLimitId, "value"),
	depEdge(apiIndexId, serverId, "value"),

	// --- @app/api → @lib/auth (value) ---
	depEdge(routeAuthId, oauthId, "value", {
		confidence: "resolverAware",
		sourceLocation: { path: "packages/api/src/routes/auth.ts", startLine: 2, endLine: 2 },
		resolutionMethod: "workspace package + tsconfig paths",
	}),
	depEdge(routeUsersId, rbacId, "value", {
		confidence: "resolverAware",
		resolutionMethod: "workspace package resolution",
	}),

	// --- @app/api → @lib/shared (value + type) ---
	depEdge(routeUsersId, commonTypesId, "typeOnly", {
		resolutionMethod: "tree-sitter import type",
	}),
	depEdge(cacheId, stringUtilId, "value"),
	depEdge(emailId, dateUtilId, "value"),
	depEdge(emailId, errorTypesId, "typeOnly"),

	// --- @lib/auth internal ---
	depEdge(authIndexId, oauthId, "value"),
	depEdge(authIndexId, jwtId, "value"),
	depEdge(authIndexId, rbacId, "value"),
	depEdge(oauthId, sessionId, "value"),
	depEdge(jwtId, sessionId, "value"),

	// --- @lib/auth → @lib/shared ---
	depEdge(oauthId, stringUtilId, "value", {
		resolutionMethod: "workspace package resolution",
	}),
	depEdge(rbacId, errorTypesId, "typeOnly", {
		resolutionMethod: "tree-sitter import type",
	}),

	// --- @lib/database internal (Rust) ---
	depEdge(dbLibId, connectionId, "normal", {
		confidence: "structural",
		resolutionMethod: "cargo_metadata",
		kind: "dependsOn",
	}),
	depEdge(dbLibId, userModelId, "normal", { kind: "dependsOn" }),
	depEdge(connectionId, dbConfigId, "value", {
		confidence: "syntactic",
		resolutionMethod: "tree-sitter use",
	}),
	depEdge(userModelId, connectionId, "value", {
		resolutionMethod: "tree-sitter use",
	}),
	depEdge(projectModelId, connectionId, "value"),
	depEdge(migrationId, userModelId, "value"),
	depEdge(migrationId, projectModelId, "value"),

	// --- @tools/cli internal (Rust) ---
	depEdge(cliMainId, cmdInitId, "normal", { kind: "dependsOn" }),
	depEdge(cliMainId, cmdScanId, "normal", { kind: "dependsOn" }),
	depEdge(cliMainId, cmdReportId, "normal", { kind: "dependsOn" }),

	// --- @tools/cli → @lib/database (build dependency) ---
	depEdge(cmdScanId, dbLibId, "build", {
		confidence: "structural",
		resolutionMethod: "cargo_metadata",
		kind: "dependsOn",
	}),

	// --- Dev dependencies ---
	depEdge(dashId, dateUtilId, "dev", {
		sourceLocation: { path: "packages/web/src/pages/Dashboard.tsx", startLine: 5, endLine: 5 },
		resolutionMethod: "tree-sitter import (test file)",
	}),
	depEdge(validationId, stringUtilId, "dev", {
		sourceLocation: { path: "packages/shared/src/utils/validation.ts", startLine: 10, endLine: 10 },
		resolutionMethod: "tree-sitter import",
	}),
];

// ===========================================================================
// OVERLAY
// ===========================================================================

// Manual edge: email service has an undiscoverable dependency on database config
const manualEdgeId = "manual:email→dbConfig";
export const fixtureOverlayEdges: AppEdge[] = [
	{
		id: manualEdgeId,
		source: emailId,
		target: dbConfigId,
		type: "dependency",
		data: {
			category: "manual",
			kind: "manual",
			isManual: true,
			isSuppressed: false,
			isBundled: false,
			bundledEdgeIds: [],
			bundledCount: 0,
			confidence: "structural",
			edgeId: manualEdgeId,
			sourceLocation: null,
			resolutionMethod: "manual config (.codeatlas.yaml)",
			suppressionReason: null,
		},
	},
];

// Suppressed edge: the validation → stringUtil dev edge is suppressed (test-only dependency)
export const fixtureSuppressedEdgeIds: ReadonlySet<string> = new Set([
	edgeId(validationId, stringUtilId, "dev"),
]);

// ===========================================================================
// MOCK DATA — compatibility report, health, unsupported constructs,
// parse failures, unresolved imports
// ===========================================================================

/** Demo unsupported constructs (3) */
export const fixtureUnsupportedConstructs: UnsupportedConstruct[] = [
	{
		constructType: "dynamicImport",
		location: { path: "packages/auth/src/providers/oauth.ts", startLine: 42, endLine: 42 },
		impact: "Dynamic import() for lazy-loading OAuth providers — not statically resolved",
		howToAddress: "Add a manual edge in .codeatlas.yaml for the dynamically loaded module",
	},
	{
		constructType: "cfgGate",
		location: { path: "crates/database/src/pool/connection.rs", startLine: 15, endLine: 20 },
		impact: '#[cfg(feature = "postgres")] — module included assuming default features',
		howToAddress: "Specify active features in graph profile or .codeatlas.yaml",
	},
	{
		constructType: "exportsCondition",
		location: { path: "packages/shared/src/types/api-types.ts", startLine: 1, endLine: 1 },
		impact: "Package uses exports conditions (import/require) — not evaluated in POC",
		howToAddress: "Full exports condition resolution available in MVP",
	},
];

/** Demo parse failure (1) */
export const fixtureParseFailures: ParseFailure[] = [
	{
		path: "tools/cli/src/commands/format-broken.rs",
		reason: "Unexpected token at line 23: unclosed brace after `fn format_output`",
	},
];

/** Demo unresolved imports (3) */
export const fixtureUnresolvedImports: UnresolvedImport[] = [
	{
		specifier: "lodash-es",
		sourceFile: "packages/web/src/pages/Dashboard.tsx",
		reason: { type: "externalPackage" },
	},
	{
		specifier: "@internal/analytics",
		sourceFile: "packages/api/src/services/cache.ts",
		reason: { type: "pathAliasNotMatched" },
	},
	{
		specifier: "import('./heavy-module')",
		sourceFile: "packages/auth/src/providers/oauth.ts",
		reason: { type: "dynamicImport" },
	},
];

/** Demo graph health — matches the fixture data */
export const fixtureGraphHealth: GraphHealth = {
	totalNodes: 64,
	resolvedEdges: 51,
	unresolvedImports: 3,
	parseFailures: 1,
	unsupportedConstructs: 3,
	unresolvedImportDetails: fixtureUnresolvedImports,
};

/** Demo compatibility report — mixed Rust + TypeScript workspace */
export const fixtureCompatibilityReport: CompatibilityReport = {
	assessments: [
		{
			language: "typescript",
			status: "partial",
			details: [
				{
					feature: "ESM imports",
					status: "supported",
					explanation: "Static import/export statements fully analyzed via tree-sitter",
				},
				{
					feature: "import type / type-only imports",
					status: "supported",
					explanation: "Type-only imports detected and classified as typeOnly edge category",
				},
				{
					feature: "tsconfig paths",
					status: "supported",
					explanation: "Path aliases resolved via tsconfig.json compilerOptions.paths",
				},
				{
					feature: "Workspace package resolution",
					status: "supported",
					explanation: "Bare specifiers matching workspace packages resolved correctly",
				},
				{
					feature: "Dynamic imports",
					status: "partial",
					explanation: "1 dynamic import() call detected — badged, not resolved",
				},
				{
					feature: "Package exports conditions",
					status: "partial",
					explanation: "1 package uses exports conditions — not evaluated in POC",
				},
				{
					feature: "CommonJS require()",
					status: "unsupported",
					explanation: "require() calls detected but not resolved in POC scope",
				},
			],
		},
		{
			language: "rust",
			status: "partial",
			details: [
				{
					feature: "Cargo workspace discovery",
					status: "supported",
					explanation: "cargo_metadata correctly discovers workspace members and dependencies",
				},
				{
					feature: "Module declarations (mod)",
					status: "supported",
					explanation: "mod declarations parsed via tree-sitter, module hierarchy built",
				},
				{
					feature: "use declarations",
					status: "supported",
					explanation: "use paths resolved to crate-internal and inter-crate targets",
				},
				{
					feature: "Dependency kinds (normal/dev/build)",
					status: "supported",
					explanation: "Edge categories from cargo_metadata dep_kinds",
				},
				{
					feature: "#[cfg(...)] gates",
					status: "partial",
					explanation: "1 #[cfg] gate detected — modules included assuming default features",
				},
				{
					feature: "build.rs code generation",
					status: "unsupported",
					explanation: "Build scripts not executed — generated code not analyzed",
				},
			],
		},
	],
	isProvisional: false,
};
