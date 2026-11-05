import { activity, agents, extensions, findModel, findProfile, findProvider, models, profiles, providers, system, type AgentFixture } from './fixtures';
import type { ActivityView, AgentView, ExtensionView, NodeKey, NodeState, Text, ToolView } from './view';
import type {
  AccountProfile,
  AgentId,
  AiDoctorConsultation,
  SessionSummary,
  SyncCycleResult,
  SyncDeviceSummary,
  UsageInsights,
  WorkspaceView,
} from '../app/backend';

// Explicit demo mode only (browser preview, UI tests). Reuses the original Phase 1 fixture
// strings so the approved demo reads exactly as before.
const checkKeys: Partial<Record<NodeKey, string>> = { install: 'installCheck', runtime: 'runtimeCheck', config: 'configCheck' };

function nodes(agent: AgentFixture): AgentView['nodes'] {
  const profile = findProfile(agent.profileId);
  const model = findModel(agent.modelId);
  return (['install', 'runtime', 'config', 'model', 'online'] as NodeKey[]).map(key => {
    let state: NodeState;
    if (key === 'online') state = 'unchecked';
    else if (key === 'model') state = !agent.installed ? 'unreached' : profile && agent.appliedRevision !== profile.revision ? 'pending' : model ? 'set' : 'unreached';
    else {
      const check = agent.checks.find(item => item.key === checkKeys[key]);
      state = check ? check.result === 'pass' ? 'pass' : 'fail' : 'unreached';
    }
    const meta: Text = {
      install: agent.version ? { key: agent.version, literal: true } : { key: 'agents:noRecord' },
      runtime: agent.checks.some(check => check.key === 'runtimeCheck') ? { key: 'agents:runtimeCheck' } : { key: 'health:nodeUnreached' },
      config: { key: 'agents:configCheck' },
      model: profile ? { key: 'health:revisionChange', params: { applied: agent.appliedRevision ?? '?', desired: profile.revision } } : model ? { key: model.identifier, literal: true } : { key: 'agents:noRecord' },
      online: { key: 'health:remoteUnknown' },
    }[key];
    return { key, state, meta };
  });
}

function extensionView(item: typeof extensions[number]): ExtensionView {
  return {
    id: item.id, type: item.type, name: { key: `extensions:${item.nameKey}` }, literalName: false, readiness: item.readiness,
    status: { key: item.installed ? 'common:installed' : 'common:available' }, scope: { key: `common:${item.scope}` },
    permission: { key: `extensions:${item.permissionKey}` }, agents: item.agents,
  };
}

export function demoAgents(): AgentView[] {
  return agents.map(agent => {
    const model = findModel(agent.modelId);
    const profile = findProfile(agent.profileId);
    return {
      id: agent.id, name: agent.name, vendor: agent.vendor, installed: agent.installed, readiness: agent.readiness, version: agent.version,
      modelId: model?.identifier ?? null, modelSource: { key: `agents:${agent.assignment}` }, assignment: agent.assignment,
      providerName: findProvider(model?.providerId)?.name ?? null,
      profileName: profile ? { key: `models:${profile.nameKey}` } : null,
      desiredRevision: profile?.revision ?? null, appliedRevision: agent.appliedRevision, pending: profile ? agent.appliedRevision !== profile.revision : false,
      nodes: nodes(agent),
      checks: agent.checks.map(check => ({ id: check.key, label: { key: `agents:${check.key}` }, result: { key: check.result === 'pass' ? 'agents:checkPass' : 'agents:checkFail' }, state: check.result, required: true, evidence: [] })),
      requiredPassed: agent.checks.filter(check => check.result === 'pass').length, requiredTotal: agent.checks.length,
      headline: { key: `agents:${agent.id}Issue` }, summary: { key: `agents:${agent.id}Summary` }, next: { key: `agents:${agent.id}Next` }, context: { key: `home:${agent.id}Context` },
      findings: [], extensions: extensions.filter(item => agent.extensionIds.includes(item.id)).map(extensionView),
      observedAt: '2026-10-07T02:30:00Z', stale: false,
    };
  });
}

export const demoExtensions = (): ExtensionView[] => extensions.map(extensionView);
export const demoTools = (): ToolView[] => system.tools.map(tool => ({ key: tool.name, name: tool.name, version: tool.version, readiness: tool.readiness, status: { key: tool.readiness === 'unknown' ? 'system:evidenceMissing' : 'system:runtimeReady' }, agentIds: tool.agentIds, location: null }));
export const demoActivity = (): ActivityView[] => activity.map(item => ({ id: item.id, text: { key: `home:${item.key}` }, time: item.time }));
export const demoSystem = { os: system.os, arch: system.architecture };
export const demoCatalog = { models, profiles, providers };
export const demoWorkspaces = (): WorkspaceView[] => [
  {
    id: 'demo_ws_1',
    name: 'Nexus Core',
    path: '/Users/developer/Documents/Nexus',
    displayPath: '~/Documents/Nexus',
    primaryAgent: 'claude',
    fallbackAgent: 'codex',
    activeAgent: 'claude',
    readiness: 'ready',
    readinessReasons: ['workspace.agent_ready:claude'],
    instructions: [
      {
        fileName: 'AGENTS.md',
        relativePath: 'AGENTS.md',
        sizeBytes: 4200,
        lineCount: 84,
        targetAgent: 'universal',
        isSupportedByPrimary: true,
      },
      {
        fileName: 'CLAUDE.md',
        relativePath: 'CLAUDE.md',
        sizeBytes: 1540,
        lineCount: 32,
        targetAgent: 'claude',
        isSupportedByPrimary: true,
      },
    ],
    isGitRepo: true,
    gitBranch: 'main',
    createdAt: '2026-10-27T00:00:00Z',
    updatedAt: '2026-11-05T00:00:00Z',
    lastOpenedAt: '2026-11-05T06:00:00Z',
  },
];

export const demoAiDoctor = (agentId: AgentId): AiDoctorConsultation => ({
  agentId,
  consultedAt: '2026-11-05T07:30:00Z',
  overallSeverity: agentId === 'codex' ? 'advisory' : 'healthy',
  confidenceScore: 0.96,
  analysisSummary: agentId === 'codex'
    ? 'Agent Codex is operational with 1 advisory warning. Recommended Model Profile assignment is available.'
    : `Agent ${agentId} is completely healthy and ready to work. All 7 lifecycle stages are synchronized and verified.`,
  radarStages: [
    { id: 'runtime', label: 'Runtime Environment', outcome: 'pass', summary: 'Runtime Environment verified and operational', checkIds: ['platform.supported', 'runtime.prerequisite'] },
    { id: 'toolchain', label: 'Toolchain & Binary', outcome: 'pass', summary: 'Toolchain & Binary verified and operational', checkIds: ['install.present', 'install.executable', 'install.version'] },
    { id: 'config', label: 'Config & Integrity', outcome: 'pass', summary: 'Config & Integrity verified and operational', checkIds: ['config.unambiguous'] },
    { id: 'auth', label: 'Auth & Tokens', outcome: 'pass', summary: 'Auth & Tokens verified and operational', checkIds: ['auth.evidence'] },
    { id: 'model', label: 'Model & Drift', outcome: agentId === 'codex' ? 'unknown' : 'pass', summary: agentId === 'codex' ? 'Model & Drift state unverified' : 'Model & Drift verified and operational', checkIds: ['model.effective', 'model.drift'] },
    { id: 'extensions', label: 'MCP & Skills', outcome: 'pass', summary: 'MCP & Skills verified and operational', checkIds: ['extensions.mcp', 'extensions.skills'] },
    { id: 'network', label: 'Provider & Network', outcome: 'pass', summary: 'Provider & Network verified and operational', checkIds: ['network.reachability'] },
  ],
  rankedCauses: [
    {
      rank: 1,
      category: agentId === 'codex' ? 'ModelAssignment' : 'Readiness',
      cause: agentId === 'codex'
        ? 'Effective agent model has drifted from the assigned Model Profile.'
        : 'All 7 lifecycle stages (Runtime, Toolchain, Config, Auth, Model, Extensions, Network) are verified.',
      confidence: 'high',
      evidenceSnippet: agentId === 'codex'
        ? 'Effective: gpt-4o, Desired: o3-mini'
        : 'Zero configuration drift and complete prerequisite coverage observed.',
    },
  ],
  recommendations: agentId === 'codex' ? [
    {
      id: 'rec_apply_model_codex',
      title: 'Apply desired Model Profile',
      rationale: 'Synchronizes the local agent configuration with your designated Model Profile.',
      riskAssessment: 'Low (Modifies model field only)',
      action: { kind: 'applyModel', agentId: 'codex' },
    },
  ] : [],
  redactionNotice: '100% of sensitive paths, tokens, and credentials were scrubbed locally before analysis.',
  disclaimer: 'AI Doctor recommendations map strictly to allowlisted native actions. Native confirmation is required before any state changes.',
});

export const demoSessions = (): SessionSummary[] => [
  {
    id: 'demo_sess_1',
    agentId: 'claude',
    workspaceId: 'demo_ws_1',
    externalSessionId: 'claude-sess-alpha',
    title: 'Visual Health Radar & AI Doctor Integration',
    projectPath: '/Users/developer/Documents/Nexus',
    displayPath: '~/Documents/Nexus',
    startedAt: '2026-11-05T06:30:00Z',
    lastActiveAt: '2026-11-05T07:15:00Z',
    messageCount: 18,
    inputTokens: 24500,
    outputTokens: 4800,
    totalTokens: 29300,
    estimatedCostCents: 15,
    estimatedCostUsd: '$0.15',
    costProvenance: 'pricing.official:Claude 3.5 Sonnet',
    modelIdentifier: 'claude-3-5-sonnet',
    canResume: true,
    resumeCommandPreview: 'claude --resume claude-sess-alpha',
  },
  {
    id: 'demo_sess_2',
    agentId: 'codex',
    workspaceId: 'demo_ws_1',
    externalSessionId: 'th_codex_982',
    title: 'SQLite Migration & Store Performance Testing',
    projectPath: '/Users/developer/Documents/Nexus',
    displayPath: '~/Documents/Nexus',
    startedAt: '2026-11-05T05:00:00Z',
    lastActiveAt: '2026-11-05T05:40:00Z',
    messageCount: 12,
    inputTokens: 16000,
    outputTokens: 3200,
    totalTokens: 19200,
    estimatedCostCents: 7,
    estimatedCostUsd: '$0.07',
    costProvenance: 'pricing.official:GPT-4o',
    modelIdentifier: 'gpt-4o',
    canResume: true,
    resumeCommandPreview: 'codex resume th_codex_982',
  },
];

export const demoUsageInsights = (): UsageInsights => ({
  totalInputTokens: 40500,
  totalOutputTokens: 8000,
  totalTokens: 48500,
  totalEstimatedCostUsd: '$0.22',
  totalSessionsCount: 2,
  byAgent: [
    {
      agentId: 'claude',
      sessionCount: 1,
      totalTokens: 29300,
      inputTokens: 24500,
      outputTokens: 4800,
      estimatedCostUsd: '$0.15',
      percentageOfTotal: 60.4,
    },
    {
      agentId: 'codex',
      sessionCount: 1,
      totalTokens: 19200,
      inputTokens: 16000,
      outputTokens: 3200,
      estimatedCostUsd: '$0.07',
      percentageOfTotal: 39.6,
    },
  ],
  byModel: [
    {
      modelIdentifier: 'claude-3-5-sonnet',
      sessionCount: 1,
      inputTokens: 24500,
      outputTokens: 4800,
      totalTokens: 29300,
      estimatedCostUsd: '$0.15',
      isEstimated: true,
    },
    {
      modelIdentifier: 'gpt-4o',
      sessionCount: 1,
      inputTokens: 16000,
      outputTokens: 3200,
      totalTokens: 19200,
      estimatedCostUsd: '$0.07',
      isEstimated: true,
    },
  ],
  byWorkspace: [
    {
      workspaceId: 'demo_ws_1',
      workspaceName: 'Nexus Core',
      sessionCount: 2,
      totalTokens: 48500,
      estimatedCostUsd: '$0.22',
    },
  ],
  provenanceNotes: [
    'usage.provenance.local_only',
    'usage.provenance.official_rates',
    'usage.provenance.non_billing_estimate',
  ],
});

export const demoAccount = (): AccountProfile => ({
  id: 'acc_demo_pro',
  email: 'developer@nexus.dev',
  displayName: 'Lead Engineer',
  tier: 'pro',
  avatarUrl: null,
  createdAt: '2026-10-27T00:00:00Z',
  expiresAt: '2027-10-27T00:00:00Z',
  backupRetentionDays: 90,
  cloudSyncEnabled: true,
  isOfflineOnly: false,
});

export const demoSyncDevices = (): SyncDeviceSummary[] => [
  {
    deviceId: 'dev_mac_primary',
    deviceName: 'MacBook Pro 16" (M3 Max)',
    platform: 'macos',
    pairedAt: '2026-10-27T00:00:00Z',
    lastSyncedAt: '2026-11-05T08:30:00Z',
    isCurrent: true,
    status: 'synced',
  },
  {
    deviceId: 'dev_win_desktop',
    deviceName: 'Studio Workstation (Windows 11)',
    platform: 'windows',
    pairedAt: '2026-10-29T12:00:00Z',
    lastSyncedAt: '2026-11-05T07:45:00Z',
    isCurrent: false,
    status: 'synced',
  },
];

export const demoSyncResult = (): SyncCycleResult => ({
  envelopesSynced: 4,
  devicesContacted: 2,
  conflicts: [],
  timestamp: '2026-11-05T08:30:00Z',
  provenance: 'Client-Side Encrypted via AES-256-GCM · Zero Plain Credentials Synced',
});
