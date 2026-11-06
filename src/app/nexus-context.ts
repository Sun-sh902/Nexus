import { createContext, useCallback, useContext } from 'react';
import { useTranslation } from 'react-i18next';
import type {
  AccountProfile,
  AccountTier,
  AgentId,
  AiDoctorConsultation,
  ConflictResolutionStrategy,
  CreateWorkspaceRequest,
  DiscoveredWorkspaceCandidate,
  ExtensionPermissionAnalysis,
  GovernanceStateView,
  LivePulseReport,
  ModelsView,
  NexusError,
  OrgRole,
  Overview,
  RegistryExtension,
  RegistrySyncSummary,
  SessionSummary,
  SignatureVerificationResult,
  SyncCycleResult,
  SyncDeviceSummary,
  UpdateOrgPolicyRequest,
  UsageInsights,
  WorkspaceView,
} from './backend';
import type { ActivityView, AgentView, ExtensionView, Text, ToolView } from '../data/view';

export type DataMode = 'live' | 'demo';
export interface NexusData {
  mode: DataMode;
  status: 'loading' | 'ready' | 'error';
  error: NexusError | null;
  overview: Overview | null;
  agents: AgentView[];
  extensions: ExtensionView[];
  tools: ToolView[];
  activity: ActivityView[];
  workspaces: WorkspaceView[];
  sessions: SessionSummary[];
  usageInsights: UsageInsights | null;
  scannedAt: string | null;
  refreshing: boolean;
  refresh: () => Promise<void>;
  models: ModelsView | null;
  modelsError: NexusError | null;
  reloadModels: () => Promise<void>;
  reloadWorkspaces: () => Promise<void>;
  createWorkspace: (req: CreateWorkspaceRequest) => Promise<WorkspaceView>;
  deleteWorkspace: (id: string) => Promise<void>;
  launchWorkspace: (id: string) => Promise<string>;
  consultAiDoctor: (agentId: AgentId) => Promise<AiDoctorConsultation>;
  reloadSessions: (workspaceId?: string, agentId?: AgentId) => Promise<void>;
  resumeSession: (id: string) => Promise<string>;
  deleteSession: (id: string) => Promise<void>;
  reloadUsageInsights: (workspaceId?: string, agentId?: AgentId) => Promise<void>;
  account: AccountProfile | null;
  syncDevices: SyncDeviceSummary[];
  syncStatus: SyncCycleResult | null;
  syncing: boolean;
  reloadAccount: () => Promise<void>;
  switchAccountTier: (tier: AccountTier) => Promise<void>;
  triggerSync: () => Promise<SyncCycleResult>;
  resolveSyncConflict: (conflictId: string, strategy: ConflictResolutionStrategy) => Promise<void>;
  exportAccountBundle: () => Promise<string>;
  purgeCloudSync: () => Promise<void>;
  registryExtensions: RegistryExtension[];
  registrySyncSummary: RegistrySyncSummary | null;
  registryLoading: boolean;
  reloadRegistry: (query?: string, tier?: string) => Promise<void>;
  analyzePermissions: (extensionId: string, agentId: AgentId) => Promise<ExtensionPermissionAnalysis>;
  verifySignature: (extensionId: string) => Promise<SignatureVerificationResult>;
  refreshRegistry: (force?: boolean) => Promise<void>;
  governance: GovernanceStateView | null;
  reloadGovernance: () => Promise<void>;
  switchOrgRole: (role: OrgRole) => Promise<void>;
  updateOrgPolicy: (req: UpdateOrgPolicyRequest) => Promise<void>;
  triggerEmergencyBreakGlass: (enabled: boolean, reason?: string) => Promise<void>;
  livePulse: LivePulseReport | null;
  reloadLivePulse: () => Promise<void>;
  discoveredWorkspaces: DiscoveredWorkspaceCandidate[];
  discoverLocalWorkspaces: () => Promise<DiscoveredWorkspaceCandidate[]>;
  findAgent: (id: string) => AgentView | undefined;
}


export const NexusContext = createContext<NexusData | null>(null);

export function useNexus(): NexusData {
  const value = useContext(NexusContext);
  if (!value) throw new Error('NexusDataProvider missing');
  return value;
}

/** Renders a view-model `Text`. Literal values are shown verbatim; `$t:` params are translated. */
export function useText() {
  const { t } = useTranslation();
  return useCallback((text: Text | null | undefined): string => {
    if (!text) return '';
    if (text.literal) return text.key;
    const params = Object.fromEntries(Object.entries(text.params ?? {}).map(([k, v]) => [k, typeof v === 'string' && v.startsWith('$t:') ? t(v.slice(3)) : v]));
    return t(text.key, params);
  }, [t]);
}
