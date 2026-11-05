import { invoke, isTauri } from '@tauri-apps/api/core';

// Typed mirror of the Rust IPC contract (src-tauri/src/ipc.rs). Rust validates every input;
// these types only describe the shapes. Pairs arrive as [key, value] tuples from Rust.
export type Pairs = [string, string][];
export type AgentId = 'codex' | 'claude' | 'gemini' | 'opencode';
export type Readiness = 'ready' | 'attention' | 'blocked' | 'unknown' | 'unsupported';
export type ObsStatus = 'observed' | 'notFound' | 'accessDenied' | 'timeout' | 'parseFailed' | 'notChecked' | 'unsafe';
export type Support = 'supported' | 'unverified' | 'unsupported';

export interface Observed<T> { value: T | null; status: ObsStatus; source: string; location: string | null; observedAt: string }
export interface Installation { id: string; executable: string; resolved: string | null; source: string; version: Observed<string>; pathRank: number | null; executableOk: boolean; nexusManaged: boolean; wrapper: boolean }
export interface ConfigSource { id: string; scope: 'user' | 'managed' | 'agentState'; format: string; location: string; status: ObsStatus; fingerprint: string | null; viaSymlink: boolean; tolerant: boolean }
export interface ModelLayer { kind: string; location: string; value: string | null; effective: boolean; uncertain: boolean }
export interface McpServer { name: string; transport: string; enabled: boolean | null; command: string | null; commandResolvable: boolean | null; endpointOrigin: string | null; envNames: string[]; scope: string; location: string; nexusManaged: boolean; canToggle: boolean }
export interface Skill { name: string; description: string | null; location: string; hasScripts: boolean; valid: boolean; nexusManaged?: boolean }
export interface Capability { key: string; support: Support; reason: string | null }
export interface AgentSnapshot {
  snapshotId: string; agentId: AgentId; name: string; vendor: string; observedAt: string; platform: Support;
  installations: Installation[]; primary: number | null; configSources: ConfigSource[];
  model: { effective: string | null; effectiveKind: string; layers: ModelLayer[]; providerHint: string | null };
  auth: { evidence: string[]; locations: string[] };
  extensions: { mcp: McpServer[]; mcpStatus: ObsStatus; skills: Skill[]; skillsStatus: ObsStatus };
  envOverrides: { variable: string; origin: string; location: string | null; affects: string }[];
  capabilities: Capability[];
  runtimeRequirement: { tool: string; minimum: string | null; evidence: string } | null;
}
export interface HealthCheck { id: string; category: string; requirement: 'required' | 'advisory'; outcome: 'pass' | 'fail' | 'unknown' | 'notApplicable'; detail: string | null; params: Pairs; evidence: string[]; source: string | null; evaluatedAt: string }
export interface AgentHealth { agentId: AgentId; readiness: Readiness; checks: HealthCheck[]; coverage: { required: number; requiredPassed: number; advisory: number; advisoryPassed: number; unknown: number; notChecked: string[] }; ruleSet: string; evaluatedAt: string; validUntil: string; stale: boolean }
export interface RepairOption { action: string; params: Pairs }
export interface Finding { id: string; code: string; agentId: AgentId; checkId: string | null; severity: 'blocking' | 'warning' | 'unknown'; cause: 'established' | 'possible'; params: Pairs; evidence: string[]; observedAt: string; stale: boolean; repairs: RepairOption[]; manual: string | null }
export interface ProviderProfile { id: string; displayName: string; kind: string; baseUrl: string | null; authMode: 'agentLogin' | 'nexusSecret' | 'environment'; secretRef: string | null; secretPresent: boolean | null; envVar: string | null; revision: number; builtin: boolean; connection: string }
export interface ProviderReachabilityResult { providerId: string; status: 'connected' | 'authFailed' | 'unreachable' | 'invalidEndpoint' | 'notConfigured'; httpStatus?: number; latencyMs?: number; message: string; modelsDiscovered: string[] }
export interface ModelDefinition { id: string; providerId: string; identifier: string; displayName: string | null; source: 'builtin' | 'observed' | 'manual'; sourceDetail: string | null; capabilities: { key: string; state: string; source: string }[] }
export interface ModelProfile { id: string; name: string; revision: number; modelId: string; reasoning: string | null }
export interface Compatibility { agentId: AgentId; support: Support; reason: string | null; nativeIdentifier: string | null }
export interface Assignment { agentId: AgentId; mode: 'native' | 'direct' | 'profile'; modelId: string | null; profileId: string | null; desiredRevision: number | null; applied: { identifier: string; profileRevision: number | null; providerRevision: number | null; appliedAt: string; location: string; fingerprint: string } | null; updatedAt: string }
export interface ResolvedAssignment { assignment: Assignment; provider: ProviderProfile; model: ModelDefinition; profile: ModelProfile | null; profileRevision: number | null; compatibility: Compatibility; pending: boolean }
export interface LifecycleOption { recipeId: string; op: 'install' | 'update' | 'uninstall'; package: string; available: boolean; installationId: string | null }
export interface AgentReport { snapshot: AgentSnapshot; health: AgentHealth; findings: Finding[]; assignment: ResolvedAssignment | null; lifecycle: LifecycleOption[] }
export interface ToolObservation { key: string; name: string; present: boolean; location: string | null; version: Observed<string>; candidates: number }
export interface SystemSnapshot { observedAt: string; os: 'macos' | 'windows' | 'linux' | 'other'; osVersion: Observed<string>; arch: string; shell: string | null; tools: ToolObservation[]; pathEntries: { location: string; exists: boolean; onPath: boolean }[]; dataDir: string; dataDirWritable: boolean; guiPathNote: boolean }
export interface OperationRecord { id: string; action: string; agentId: string | null; status: string; startedAt: string; finishedAt: string | null; errorCode: string | null; summary: Record<string, unknown>; steps: string[] }
export interface Overview { contractVersion: 1; generatedAt: string; system: SystemSnapshot; agents: AgentReport[]; recentOperations: OperationRecord[] }
export interface ModelsView { providers: ProviderProfile[]; models: ModelDefinition[]; profiles: ModelProfile[]; assignments: ResolvedAssignment[]; matrix: [string, Compatibility[]][] }
export interface BackupRecord { id: string; operationId: string; agentId: string | null; targetId: string; targetDisplay: string; sha256: string; size: number; createdAt: string }
export interface AgentHistory { backups: BackupRecord[]; operations: OperationRecord[]; health: { agentId: string; readiness: Readiness; issueCodes: string[]; evaluatedAt: string }[] }
export interface StorageStatus { dataDir: string; dbBytes: number; backupCount: number; backupBytes: number; operationCount: number; unresolvedCount: number; healthRowCount: number }
export interface PruneResult { prunedOperations: number; prunedHealthRows: number; prunedBackups: number }
export interface DiagnosticPreview { filename: string; json: string }

export type WorkspaceReadiness = 'ready' | 'attention' | 'blocked';

export interface WorkspaceInstruction {
  fileName: string;
  relativePath: string;
  sizeBytes: number;
  lineCount: number;
  targetAgent: string;
  isSupportedByPrimary: boolean;
}

export interface WorkspaceView {
  id: string;
  name: string;
  path: string;
  displayPath: string;
  primaryAgent: AgentId;
  fallbackAgent?: AgentId;
  activeAgent: AgentId;
  readiness: WorkspaceReadiness;
  readinessReasons: string[];
  instructions: WorkspaceInstruction[];
  isGitRepo: boolean;
  gitBranch?: string;
  createdAt: string;
  updatedAt: string;
  lastOpenedAt?: string;
}

export interface CreateWorkspaceRequest {
  name?: string;
  path: string;
  primaryAgent: AgentId;
  fallbackAgent?: AgentId;
  modelId?: string;
  profileId?: string;
}

export type ConfidenceLevel = 'high' | 'medium' | 'low';

export interface ProbableCause {
  rank: number;
  category: string;
  cause: string;
  confidence: ConfidenceLevel;
  evidenceSnippet: string;
}

export interface AiDoctorRecommendation {
  id: string;
  title: string;
  rationale: string;
  riskAssessment: string;
  action: ActionRequest | null;
}

export interface HealthRadarStage {
  id: string;
  label: string;
  outcome: 'pass' | 'fail' | 'unknown' | 'notApplicable';
  summary: string;
  checkIds: string[];
}

export interface AiDoctorConsultation {
  agentId: AgentId;
  consultedAt: string;
  overallSeverity: 'healthy' | 'advisory' | 'critical';
  confidenceScore: number;
  analysisSummary: string;
  radarStages: HealthRadarStage[];
  rankedCauses: ProbableCause[];
  recommendations: AiDoctorRecommendation[];
  redactionNotice: string;
  disclaimer: string;
}

export interface SessionSummary {
  id: string;
  agentId: AgentId;
  workspaceId?: string;
  externalSessionId: string;
  title: string;
  projectPath: string;
  displayPath: string;
  startedAt: string;
  lastActiveAt: string;
  messageCount: number;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  estimatedCostCents: number;
  estimatedCostUsd: string;
  costProvenance: string;
  modelIdentifier?: string;
  canResume: boolean;
  resumeCommandPreview: string;
}

export interface AgentUsageBreakdown {
  agentId: AgentId;
  sessionCount: number;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  estimatedCostUsd: string;
  percentageOfTotal: number;
}

export interface ModelUsageBreakdown {
  modelIdentifier: string;
  sessionCount: number;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  estimatedCostUsd: string;
  isEstimated: boolean;
}

export interface WorkspaceUsageBreakdown {
  workspaceId: string;
  workspaceName: string;
  sessionCount: number;
  totalTokens: number;
  estimatedCostUsd: string;
}

export interface UsageInsights {
  totalInputTokens: number;
  totalOutputTokens: number;
  totalTokens: number;
  totalEstimatedCostUsd: string;
  totalSessionsCount: number;
  byAgent: AgentUsageBreakdown[];
  byModel: ModelUsageBreakdown[];
  byWorkspace: WorkspaceUsageBreakdown[];
  provenanceNotes: string[];
}

export type AccountTier = 'free' | 'pro' | 'team';

export interface AccountProfile {
  id: string;
  email: string;
  displayName: string;
  tier: AccountTier;
  avatarUrl?: string | null;
  createdAt: string;
  expiresAt?: string | null;
  backupRetentionDays: number;
  cloudSyncEnabled: boolean;
  isOfflineOnly: boolean;
}

export interface EntitlementStatus {
  tier: AccountTier;
  isValid: boolean;
  features: string[];
  retentionDays: number;
  maxOperations: number;
  safetyGuarantee: string;
}

export type SyncStatus = 'synced' | 'syncing' | 'offline' | 'conflict';

export interface SyncDeviceSummary {
  deviceId: string;
  deviceName: string;
  platform: 'macos' | 'windows' | 'linux';
  pairedAt: string;
  lastSyncedAt: string;
  isCurrent: boolean;
  status: SyncStatus;
}

export interface SyncConflict {
  conflictId: string;
  entityType: string;
  entityId: string;
  localVersion: number;
  localChecksum: string;
  remoteDeviceName: string;
  remoteVersion: number;
  remoteSyncedAt: string;
  remoteChecksum: string;
  description: string;
}

export interface SyncCycleResult {
  envelopesSynced: number;
  devicesContacted: number;
  conflicts: SyncConflict[];
  timestamp: string;
  provenance: string;
}

export type ConflictResolutionStrategy = 'local_wins' | 'remote_wins' | 'merge_newest';

export type TrustTier = 'official' | 'verified_partner' | 'community';
export type ReviewLevel = 'verified_official' | 'verified_partner' | 'community_reviewed' | 'experimental' | 'revoked';
export type SignatureStatus = 'valid' | 'unsigned' | 'invalid_signature' | 'revoked';
export type ExtensionPermissionType = 'filesystem_read' | 'filesystem_write' | 'network_access' | 'environment_variables' | 'process_execution';

export interface ExtensionPermission {
  kind: ExtensionPermissionType;
  description: string;
  target: string;
  critical: boolean;
}

export interface PermissionDelta {
  permission: ExtensionPermission;
  change: 'added' | 'removed' | 'unchanged';
  isPrivilegeEscalation: boolean;
}

export interface RegistryPublisher {
  publisherId: string;
  name: string;
  displayName: string;
  verified: boolean;
  trustTier: TrustTier;
  publicKeyFingerprint: string;
  websiteUrl?: string | null;
  supportUrl?: string | null;
  createdAt: string;
}

export interface RegistryExtension {
  id: string;
  publisher: RegistryPublisher;
  name: string;
  displayName: string;
  description: string;
  extensionType: 'mcp' | 'skill';
  version: string;
  reviewLevel: ReviewLevel;
  signatureStatus: SignatureStatus;
  manifestSha256: string;
  signatureEd25519: string;
  permissions: ExtensionPermission[];
  compatibleAgents: AgentId[];
  downloadSize: number;
  revocationReason?: string | null;
  updatedAt: string;
  installed: boolean;
}

export interface ExtensionPermissionAnalysis {
  extensionId: string;
  targetAgent: AgentId;
  existingPermissions: ExtensionPermission[];
  requestedPermissions: ExtensionPermission[];
  deltas: PermissionDelta[];
  hasPrivilegeEscalation: boolean;
  requiresExplicitConfirmation: boolean;
  isRevoked: boolean;
  revocationReason?: string | null;
}

export interface SignatureVerificationResult {
  extensionId: string;
  publisherId: string;
  keyFingerprint: string;
  manifestSha256: string;
  computedSha256: string;
  status: SignatureStatus;
  verifiedAt: string;
  isTrusted: boolean;
}

export interface RegistrySyncSummary {
  syncedAt: string;
  totalExtensions: number;
  revocationCount: number;
  offlineCached: boolean;
  source: string;
}
export interface ConfigEdit { targetId: string; location: string; format: string; createsFile: boolean; beforeFingerprint: string | null; afterFingerprint: string; changes: { key: string; before: string | null; after: string | null }[]; diff: { kind: '+' | '-'; text: string }[] }
export interface PlanView {
  id: string; digest: string; kind: string; agentId: AgentId | null; createdAt: string; expiresAt: string; params: Pairs;
  config: ConfigEdit | null;
  process: { program: string; args: string[]; envNames: string[]; runsPackageScripts: boolean; usesNetwork: boolean; package: string; version: string | null } | null;
  privileges: string; backup: string; rollback: string; verification: string; warnings: string[]; preserves: string[];
}
export interface OperationResult {
  operationId: string; kind: string; agentId: AgentId | null; status: 'succeeded' | 'failed' | 'partial' | 'rolledBack' | 'rollbackFailed';
  verification: { outcome: 'passed' | 'failed' | 'inconclusive' | 'notRun'; checks: { key: string; outcome: string; params: Pairs; informational: boolean }[]; verifiedAt: string };
  error: NexusErrorShape | null; backupId: string | null; compensation: string | null; outputExcerpt: string | null; warnings: string[];
}
export type ActionRequest =
  | { kind: 'applyModel'; agentId: AgentId }
  | { kind: 'setMcpEnabled'; agentId: AgentId; name: string; enabled: boolean }
  | { kind: 'addRemoteMcp'; agentId: AgentId; name: string; url: string }
  | { kind: 'addReviewedMcp'; agentId: AgentId; name: string; recipeId: string; directory?: string }
  | { kind: 'removeManagedMcp'; agentId: AgentId; name: string }
  | { kind: 'addSkill'; agentId: AgentId; sourceDirectory: string }
  | { kind: 'removeManagedSkill'; agentId: AgentId; name: string }
  | { kind: 'restoreBackup'; backupId: string }
  | { kind: 'install'; agentId: AgentId; recipeId: string }
  | { kind: 'update'; agentId: AgentId; installationId: string }
  | { kind: 'uninstall'; agentId: AgentId; installationId: string }
  | { kind: 'recoverInstall'; operationId: string };

export interface NexusErrorShape { code: string; params: Pairs; retryable: boolean }

/** Native errors keep their stable code; the UI translates `errors:<code>`. */
export class NexusError extends Error {
  readonly code: string;
  readonly params: Record<string, string>;
  readonly retryable: boolean;
  constructor(shape: NexusErrorShape) {
    super(shape.code);
    this.code = shape.code;
    this.params = Object.fromEntries(shape.params ?? []);
    this.retryable = shape.retryable;
  }
}

export const pairs = (value: Pairs | undefined | null): Record<string, string> => Object.fromEntries(value ?? []);
export const liveBackend = () => isTauri();

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && typeof (error as NexusErrorShape).code === 'string') throw new NexusError(error as NexusErrorShape);
    throw new NexusError({ code: 'ipc.unavailable', params: [], retryable: true });
  }
}

export const backend = {
  overview: (refresh: boolean) => call<Overview>('nexus_overview', { refresh }),
  agentHistory: (agentId: AgentId) => call<AgentHistory>('nexus_agent_history', { agentId }),
  models: () => call<ModelsView>('nexus_models'),
  createProvider: (input: { name: string; kind: string; baseUrl?: string; authMode: string; envVar?: string }) => call<ProviderProfile>('nexus_create_provider', { input }),
  setProviderSecret: (providerId: string, secret: string) => call<void>('nexus_set_provider_secret', { input: { providerId, secret } }),
  deleteProvider: (providerId: string) => call<void>('nexus_delete_provider', { providerId }),
  checkProvider: (providerId: string) => call<ProviderReachabilityResult>('nexus_check_provider', { providerId }),
  createModel: (input: { providerId: string; identifier: string; displayName?: string }) => call<ModelDefinition>('nexus_create_model', { input }),
  deleteModel: (modelId: string) => call<void>('nexus_delete_model', { modelId }),
  saveProfile: (input: { id?: string; name: string; modelId: string; reasoning?: string }) => call<ModelProfile>('nexus_save_profile', { input }),
  deleteProfile: (profileId: string) => call<void>('nexus_delete_profile', { profileId }),
  setAssignment: (input: { agentId: AgentId; mode: 'native' | 'direct' | 'profile'; modelId?: string; profileId?: string }) => call<ResolvedAssignment | null>('nexus_set_assignment', { input }),
  prepare: (request: ActionRequest) => call<PlanView>('nexus_prepare_action', { request }),
  confirm: (plan: PlanView) => call<OperationResult>('nexus_confirm_plan', { planId: plan.id, digest: plan.digest }),
  discard: (planId: string) => call<void>('nexus_discard_plan', { planId }),
  launch: (agentId: AgentId, installationId: string) => call<string>('nexus_launch_agent', { agentId, installationId }),
  storageStatus: () => call<StorageStatus>('nexus_storage_status'),
  pruneHistory: (retainDays: number) => call<PruneResult>('nexus_prune_history', { retainDays }),
  diagnosticPreview: () => call<DiagnosticPreview>('nexus_diagnostic_preview'),
  listWorkspaces: () => call<WorkspaceView[]>('nexus_list_workspaces'),
  createWorkspace: (request: CreateWorkspaceRequest) => call<WorkspaceView>('nexus_create_workspace', { request }),
  deleteWorkspace: (workspaceId: string) => call<void>('nexus_delete_workspace', { workspaceId }),
  launchWorkspace: (workspaceId: string) => call<string>('nexus_launch_workspace', { workspaceId }),
  consultAiDoctor: (agentId: AgentId) => call<AiDoctorConsultation>('nexus_consult_ai_doctor', { agentId }),
  listSessions: (workspaceId?: string, agentId?: AgentId, limit?: number) => call<SessionSummary[]>('nexus_list_sessions', { workspaceId: workspaceId ?? null, agentId: agentId ?? null, limit: limit ?? null }),
  resumeSession: (sessionId: string) => call<string>('nexus_resume_session', { sessionId }),
  deleteSession: (sessionId: string) => call<void>('nexus_delete_session', { sessionId }),
  getUsageInsights: (workspaceId?: string, agentId?: AgentId) => call<UsageInsights>('nexus_usage_insights', { workspaceId: workspaceId ?? null, agentId: agentId ?? null }),
  getAccount: () => call<AccountProfile>('nexus_get_account'),
  switchAccountTier: (tier: AccountTier) => call<AccountProfile>('nexus_switch_account_tier', { tier }),
  listSyncDevices: () => call<SyncDeviceSummary[]>('nexus_list_sync_devices'),
  triggerSync: () => call<SyncCycleResult>('nexus_trigger_sync'),
  resolveSyncConflict: (conflictId: string, strategy: ConflictResolutionStrategy) =>
    call<void>('nexus_resolve_sync_conflict', { conflictId, strategy }),
  exportAccountBundle: () => call<string>('nexus_export_account_bundle'),
  purgeCloudSync: () => call<number>('nexus_purge_cloud_sync'),
  listRegistryExtensions: (query?: string, filterTier?: string) =>
    call<RegistryExtension[]>('nexus_list_registry_extensions', { query: query ?? null, filterTier: filterTier ?? null }),
  getRegistryExtension: (extensionId: string) =>
    call<RegistryExtension | null>('nexus_get_registry_extension', { extensionId }),
  analyzeExtensionPermissions: (extensionId: string, targetAgent: AgentId) =>
    call<ExtensionPermissionAnalysis>('nexus_analyze_extension_permissions', { extensionId, targetAgent }),
  verifyExtensionSignature: (extensionId: string) =>
    call<SignatureVerificationResult>('nexus_verify_extension_signature', { extensionId }),
  refreshRegistryIndex: (force?: boolean) =>
    call<RegistrySyncSummary>('nexus_refresh_registry_index', { force: force ?? false }),
};
