/**
 * Type stubs for future `openakta:*` IPC channels. No main-process handlers in this phase.
 */

import type {
  AppearancePreferences,
  ArchivedThread,
  ConfigurationPreferences,
  EnvironmentProject,
  GeneralPreferences,
  GitPreferences,
  McpRegistry,
  McpServer,
  PersonalizationPreferences,
  UsageState,
  WorktreeRow,
  WorktreesConfig,
} from "./preferences";

export const IpcChannels = {
  preferencesGeneralGet: "openakta:preferences:general:get",
  preferencesGeneralSet: "openakta:preferences:general:set",
  preferencesAppearanceGet: "openakta:preferences:appearance:get",
  preferencesAppearanceSet: "openakta:preferences:appearance:set",
  preferencesGitGet: "openakta:preferences:git:get",
  preferencesGitSet: "openakta:preferences:git:set",
  preferencesPersonalizationGet: "openakta:preferences:personalization:get",
  preferencesPersonalizationSet: "openakta:preferences:personalization:set",
  configPolicyGet: "openakta:config:policy:get",
  configPolicySet: "openakta:config:policy:set",
  usageGet: "openakta:usage:get",
  mcpList: "openakta:mcp:list",
  mcpAdd: "openakta:mcp:add",
  mcpUpdate: "openakta:mcp:update",
  mcpRemove: "openakta:mcp:remove",
  threadsArchivedList: "openakta:threads:archived:list",
  threadsArchivedRestore: "openakta:threads:archived:restore",
  worktreesConfigGet: "openakta:worktrees:config:get",
  worktreesConfigSet: "openakta:worktrees:config:set",
  worktreesList: "openakta:worktrees:list",
  missionStatus: "openakta:mission:status",
  missionSubmit: "openakta:mission:submit",
  environmentsList: "openakta:environments:list",
  environmentsAdd: "openakta:environments:add",
} as const;

export type OpenaktaIpcChannel = (typeof IpcChannels)[keyof typeof IpcChannels];

export interface IpcRequestMap {
  [IpcChannels.preferencesGeneralGet]: void;
  [IpcChannels.preferencesGeneralSet]: GeneralPreferences;
  [IpcChannels.preferencesAppearanceGet]: void;
  [IpcChannels.preferencesAppearanceSet]: AppearancePreferences;
  [IpcChannels.preferencesGitGet]: void;
  [IpcChannels.preferencesGitSet]: GitPreferences;
  [IpcChannels.preferencesPersonalizationGet]: void;
  [IpcChannels.preferencesPersonalizationSet]: PersonalizationPreferences;
  [IpcChannels.configPolicyGet]: void;
  [IpcChannels.configPolicySet]: ConfigurationPreferences;
  [IpcChannels.usageGet]: void;
  [IpcChannels.mcpList]: void;
  [IpcChannels.mcpAdd]: McpServer;
  [IpcChannels.mcpUpdate]: Partial<McpServer> & { id: string };
  [IpcChannels.mcpRemove]: { id: string };
  [IpcChannels.threadsArchivedList]: void;
  [IpcChannels.threadsArchivedRestore]: { id: string };
  [IpcChannels.worktreesConfigGet]: void;
  [IpcChannels.worktreesConfigSet]: WorktreesConfig;
  [IpcChannels.worktreesList]: void;
  [IpcChannels.missionStatus]: void;
  [IpcChannels.missionSubmit]: unknown;
  [IpcChannels.environmentsList]: void;
  [IpcChannels.environmentsAdd]: EnvironmentProject;
}

export interface IpcResponseMap {
  [IpcChannels.preferencesGeneralGet]: GeneralPreferences;
  [IpcChannels.preferencesGeneralSet]: void;
  [IpcChannels.preferencesAppearanceGet]: AppearancePreferences;
  [IpcChannels.preferencesAppearanceSet]: void;
  [IpcChannels.preferencesGitGet]: GitPreferences;
  [IpcChannels.preferencesGitSet]: void;
  [IpcChannels.preferencesPersonalizationGet]: PersonalizationPreferences;
  [IpcChannels.preferencesPersonalizationSet]: void;
  [IpcChannels.configPolicyGet]: ConfigurationPreferences;
  [IpcChannels.configPolicySet]: void;
  [IpcChannels.usageGet]: UsageState;
  [IpcChannels.mcpList]: McpRegistry;
  [IpcChannels.mcpAdd]: void;
  [IpcChannels.mcpUpdate]: void;
  [IpcChannels.mcpRemove]: void;
  [IpcChannels.threadsArchivedList]: ArchivedThread[];
  [IpcChannels.threadsArchivedRestore]: void;
  [IpcChannels.worktreesConfigGet]: WorktreesConfig;
  [IpcChannels.worktreesConfigSet]: void;
  [IpcChannels.worktreesList]: WorktreeRow[];
  [IpcChannels.missionStatus]: unknown;
  [IpcChannels.missionSubmit]: void;
  [IpcChannels.environmentsList]: EnvironmentProject[];
  [IpcChannels.environmentsAdd]: void;
}
