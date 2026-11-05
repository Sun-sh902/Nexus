import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import {
  backend,
  liveBackend,
  NexusError,
  type AccountProfile,
  type AccountTier,
  type AgentId,
  type ConflictResolutionStrategy,
  type CreateWorkspaceRequest,
  type ModelsView,
  type Overview,
  type SessionSummary,
  type SyncCycleResult,
  type SyncDeviceSummary,
  type UsageInsights,
  type WorkspaceView,
} from './backend';
import { agentView, activityViews, toolViews } from '../data/live';
import {
  demoAccount,
  demoActivity,
  demoAgents,
  demoAiDoctor,
  demoExtensions,
  demoSessions,
  demoSyncDevices,
  demoSyncResult,
  demoTools,
  demoUsageInsights,
  demoWorkspaces,
} from '../data/demo';
import { NexusContext, type DataMode, type NexusData } from './nexus-context';

// Single data boundary for pages. Inside the desktop runtime every value comes from Rust; the
// fixture set is used only in explicit demo mode (browser preview / UI tests), which the shell
// labels as such. A failed scan shows an error state, never fixture data.

export function NexusDataProvider({ children }: { children: ReactNode }) {
  const mode: DataMode = liveBackend() ? 'live' : 'demo';
  const [overview, setOverview] = useState<Overview | null>(null);
  const [models, setModels] = useState<ModelsView | null>(null);
  const [modelsError, setModelsError] = useState<NexusError | null>(null);
  const [workspaces, setWorkspaces] = useState<WorkspaceView[]>(mode === 'demo' ? demoWorkspaces() : []);
  const [sessions, setSessions] = useState<SessionSummary[]>(mode === 'demo' ? demoSessions() : []);
  const [usageInsights, setUsageInsights] = useState<UsageInsights | null>(mode === 'demo' ? demoUsageInsights() : null);
  const [account, setAccount] = useState<AccountProfile | null>(mode === 'demo' ? demoAccount() : null);
  const [syncDevices, setSyncDevices] = useState<SyncDeviceSummary[]>(mode === 'demo' ? demoSyncDevices() : []);
  const [syncStatus, setSyncStatus] = useState<SyncCycleResult | null>(mode === 'demo' ? demoSyncResult() : null);
  const [syncing, setSyncing] = useState(false);
  const [status, setStatus] = useState<NexusData['status']>(mode === 'live' ? 'loading' : 'ready');
  const [error, setError] = useState<NexusError | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const generation = useRef(0);

  const reloadWorkspaces = useCallback(async () => {
    if (mode !== 'live') return;
    try {
      setWorkspaces(await backend.listWorkspaces());
    } catch {
      // non-fatal
    }
  }, [mode]);

  const reloadSessions = useCallback(async (workspaceId?: string, agentId?: AgentId) => {
    if (mode !== 'live') return;
    try {
      setSessions(await backend.listSessions(workspaceId, agentId));
    } catch {
      // non-fatal
    }
  }, [mode]);

  const reloadUsageInsights = useCallback(async (workspaceId?: string, agentId?: AgentId) => {
    if (mode !== 'live') return;
    try {
      setUsageInsights(await backend.getUsageInsights(workspaceId, agentId));
    } catch {
      // non-fatal
    }
  }, [mode]);

  const reloadAccount = useCallback(async () => {
    if (mode !== 'live') return;
    try {
      setAccount(await backend.getAccount());
      setSyncDevices(await backend.listSyncDevices());
    } catch {
      // non-fatal
    }
  }, [mode]);

  const switchAccountTier = useCallback(async (tier: AccountTier) => {
    if (mode === 'live') {
      const updated = await backend.switchAccountTier(tier);
      setAccount(updated);
    } else {
      setAccount(prev => prev ? { ...prev, tier, cloudSyncEnabled: tier !== 'free' } : null);
    }
  }, [mode]);

  const triggerSync = useCallback(async () => {
    setSyncing(true);
    try {
      if (mode === 'live') {
        const res = await backend.triggerSync();
        setSyncStatus(res);
        await reloadAccount();
        return res;
      }
      const mockRes = demoSyncResult();
      setSyncStatus(mockRes);
      return mockRes;
    } finally {
      setSyncing(false);
    }
  }, [mode, reloadAccount]);

  const resolveSyncConflict = useCallback(async (conflictId: string, strategy: ConflictResolutionStrategy) => {
    if (mode === 'live') {
      await backend.resolveSyncConflict(conflictId, strategy);
      await reloadAccount();
    }
  }, [mode, reloadAccount]);

  const exportAccountBundle = useCallback(async () => {
    if (mode === 'live') {
      return await backend.exportAccountBundle();
    }
    return JSON.stringify({ demoExport: true, account }, null, 2);
  }, [mode, account]);

  const purgeCloudSync = useCallback(async () => {
    if (mode === 'live') {
      await backend.purgeCloudSync();
      await reloadAccount();
    } else {
      setSyncDevices(prev => prev.filter(d => d.isCurrent));
    }
  }, [mode, reloadAccount]);

  const load = useCallback(async (refresh: boolean) => {
    if (mode !== 'live') return;
    const mine = ++generation.current;
    setRefreshing(true);
    try {
      const next = await backend.overview(refresh);
      if (mine !== generation.current) return;
      if (next.contractVersion !== 1) throw new NexusError({ code: 'ipc.contract_mismatch', params: [], retryable: false });
      setOverview(next);
      setStatus('ready');
      setError(null);
      void reloadWorkspaces();
      void reloadSessions();
      void reloadUsageInsights();
      void reloadAccount();
    } catch (caught) {
      if (mine !== generation.current) return;
      setError(caught instanceof NexusError ? caught : new NexusError({ code: 'ipc.unavailable', params: [], retryable: true }));
      setStatus('error');
    } finally {
      if (mine === generation.current) setRefreshing(false);
    }
  }, [mode, reloadWorkspaces, reloadSessions, reloadUsageInsights, reloadAccount]);

  const reloadModels = useCallback(async () => {
    if (mode !== 'live') return;
    try { setModels(await backend.models()); setModelsError(null); }
    catch (caught) { setModelsError(caught instanceof NexusError ? caught : new NexusError({ code: 'ipc.unavailable', params: [], retryable: true })); }
  }, [mode]);

  const createWorkspace = useCallback(async (req: CreateWorkspaceRequest) => {
    if (mode === 'live') {
      const created = await backend.createWorkspace(req);
      await reloadWorkspaces();
      return created;
    }
    const mock: WorkspaceView = {
      id: `ws_${Date.now()}`,
      name: req.name || 'New Workspace',
      path: req.path,
      displayPath: req.path.replace(/^\/Users\/[^/]+/, '~'),
      primaryAgent: req.primaryAgent,
      fallbackAgent: req.fallbackAgent,
      activeAgent: req.primaryAgent,
      readiness: 'ready',
      readinessReasons: [],
      instructions: [{ fileName: 'AGENTS.md', relativePath: 'AGENTS.md', sizeBytes: 1024, lineCount: 20, targetAgent: 'universal', isSupportedByPrimary: true }],
      isGitRepo: true,
      gitBranch: 'main',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
    setWorkspaces(prev => [mock, ...prev]);
    return mock;
  }, [mode, reloadWorkspaces]);

  const deleteWorkspace = useCallback(async (id: string) => {
    if (mode === 'live') {
      await backend.deleteWorkspace(id);
      await reloadWorkspaces();
    } else {
      setWorkspaces(prev => prev.filter(w => w.id !== id));
    }
  }, [mode, reloadWorkspaces]);

  const launchWorkspace = useCallback(async (id: string) => {
    if (mode === 'live') {
      return await backend.launchWorkspace(id);
    }
    return `op_mock_launch_${id}`;
  }, [mode]);

  const consultAiDoctor = useCallback(async (agentId: AgentId) => {
    if (mode === 'live') {
      return await backend.consultAiDoctor(agentId);
    }
    return demoAiDoctor(agentId);
  }, [mode]);

  const resumeSession = useCallback(async (id: string) => {
    if (mode === 'live') {
      const res = await backend.resumeSession(id);
      await reloadSessions();
      return res;
    }
    return `op_mock_resume_${id}`;
  }, [mode, reloadSessions]);

  const deleteSession = useCallback(async (id: string) => {
    if (mode === 'live') {
      await backend.deleteSession(id);
      await reloadSessions();
      await reloadUsageInsights();
    } else {
      setSessions(prev => prev.filter(s => s.id !== id));
    }
  }, [mode, reloadSessions, reloadUsageInsights]);

  // Deferred so the scan's state updates never run synchronously inside the effect body.
  useEffect(() => { void Promise.resolve().then(() => load(true)); }, [load]);

  useEffect(() => {
    if (mode !== 'live' || !overview) return;
    const expiry = Math.min(...overview.agents.map(agent => Date.parse(agent.health.validUntil)));
    if (!Number.isFinite(expiry)) return;
    const timer = window.setTimeout(() => {
      setError(new NexusError({ code: 'health.evidence_stale', params: [], retryable: true }));
      setStatus('error');
    }, Math.max(0, expiry - Date.now()));
    return () => window.clearTimeout(timer);
  }, [mode, overview]);

  const value = useMemo<NexusData>(() => {
    const agents = mode === 'demo' ? demoAgents() : overview ? overview.agents.map(agentView) : [];
    return {
      mode, status, error, overview, agents,
      extensions: mode === 'demo' ? demoExtensions() : agents.flatMap(agent => agent.extensions),
      tools: mode === 'demo' ? demoTools() : overview ? toolViews(overview) : [],
      activity: mode === 'demo' ? demoActivity() : overview ? activityViews(overview) : [],
      workspaces,
      sessions,
      usageInsights,
      account,
      syncDevices,
      syncStatus,
      syncing,
      scannedAt: mode === 'demo' ? '2026-10-07T02:30:00Z' : overview?.generatedAt ?? null,
      refreshing,
      refresh: () => load(true),
      models, modelsError, reloadModels,
      reloadWorkspaces,
      createWorkspace,
      deleteWorkspace,
      launchWorkspace,
      consultAiDoctor,
      reloadSessions,
      resumeSession,
      deleteSession,
      reloadUsageInsights,
      reloadAccount,
      switchAccountTier,
      triggerSync,
      resolveSyncConflict,
      exportAccountBundle,
      purgeCloudSync,
      findAgent: (id: string) => agents.find(agent => agent.id === id),
    };
  }, [
    mode, status, error, overview, workspaces, sessions, usageInsights, account, syncDevices, syncStatus, syncing,
    refreshing, load, models, modelsError, reloadModels, reloadWorkspaces, createWorkspace, deleteWorkspace,
    launchWorkspace, consultAiDoctor, reloadSessions, resumeSession, deleteSession, reloadUsageInsights,
    reloadAccount, switchAccountTier, triggerSync, resolveSyncConflict, exportAccountBundle, purgeCloudSync
  ]);
  return <NexusContext.Provider value={value}>{children}</NexusContext.Provider>;
}

