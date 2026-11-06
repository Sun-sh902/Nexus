import { useState } from 'react';
import {
  AlertTriangle,
  CheckCircle2,
  Eye,
  EyeOff,
  FileCheck2,
  KeyRound,
  Layers,
  Lock,
  Shield,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  Users,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  NexusError,
  type AgentId,
  type EnforcementMode,
  type OrgRole,
  type WorkspaceTemplateView,
} from '../app/backend';
import { useNexus } from '../app/nexus-context';
import { Notice } from './ui';

const ALL_AGENTS: AgentId[] = ['claude', 'codex', 'gemini', 'opencode'];
const ALL_PROVIDERS = ['anthropic', 'openai', 'google', 'openrouter'];
const ROLES: OrgRole[] = ['owner', 'admin', 'member', 'auditor'];
const MODES: EnforcementMode[] = ['advisory', 'enforced', 'strict_lockdown'];

export function GovernanceControlPanel() {
  const { t } = useTranslation(['governance', 'common', 'errors']);
  const {
    governance,
    switchOrgRole,
    updateOrgPolicy,
    triggerEmergencyBreakGlass,
    createWorkspace,
  } = useNexus();

  const [saving, setSaving] = useState(false);
  const [breakGlassReason, setBreakGlassReason] = useState('');
  const [feedback, setFeedback] = useState<{ tone: 'success' | 'error'; text: string } | null>(null);

  if (!governance) {
    return null;
  }

  const { policy, compliance, templates, auditLog, visibilityBoundary } = governance;

  async function handleRoleChange(role: OrgRole) {
    setFeedback(null);
    try {
      await switchOrgRole(role);
    } catch (caught) {
      const code = caught instanceof NexusError ? caught.code : 'ipc.unavailable';
      setFeedback({ tone: 'error', text: t(`errors:${code}`) });
    }
  }

  async function handleToggleAgent(agent: string) {
    if (!policy.canMutatePolicy) {
      setFeedback({
        tone: 'error',
        text: t('rbacLockedNotice', { role: t(`role_${policy.userRole}`) }),
      });
      return;
    }
    const nextAgents = policy.approvedAgents.includes(agent)
      ? policy.approvedAgents.filter(a => a !== agent)
      : [...policy.approvedAgents, agent];
    await handleSavePolicy({
      enforcementMode: policy.enforcementMode,
      approvedAgents: nextAgents,
      approvedProviders: policy.approvedProviders,
      approvedModels: policy.approvedModels,
      approvedExtensions: policy.approvedExtensions,
      requireSignedExtensions: policy.requireSignedExtensions,
      allowLocalBreakGlass: policy.allowLocalBreakGlass,
    });
  }

  async function handleToggleProvider(provider: string) {
    if (!policy.canMutatePolicy) {
      setFeedback({
        tone: 'error',
        text: t('rbacLockedNotice', { role: t(`role_${policy.userRole}`) }),
      });
      return;
    }
    const nextProviders = policy.approvedProviders.includes(provider)
      ? policy.approvedProviders.filter(p => p !== provider)
      : [...policy.approvedProviders, provider];
    await handleSavePolicy({
      enforcementMode: policy.enforcementMode,
      approvedAgents: policy.approvedAgents,
      approvedProviders: nextProviders,
      approvedModels: policy.approvedModels,
      approvedExtensions: policy.approvedExtensions,
      requireSignedExtensions: policy.requireSignedExtensions,
      allowLocalBreakGlass: policy.allowLocalBreakGlass,
    });
  }

  async function handleModeChange(nextMode: EnforcementMode) {
    if (!policy.canMutatePolicy) {
      setFeedback({
        tone: 'error',
        text: t('rbacLockedNotice', { role: t(`role_${policy.userRole}`) }),
      });
      return;
    }
    await handleSavePolicy({
      enforcementMode: nextMode,
      approvedAgents: policy.approvedAgents,
      approvedProviders: policy.approvedProviders,
      approvedModels: policy.approvedModels,
      approvedExtensions: policy.approvedExtensions,
      requireSignedExtensions: policy.requireSignedExtensions,
      allowLocalBreakGlass: policy.allowLocalBreakGlass,
    });
  }

  async function handleToggleFlag(field: 'requireSignedExtensions' | 'allowLocalBreakGlass') {
    if (!policy.canMutatePolicy) {
      setFeedback({
        tone: 'error',
        text: t('rbacLockedNotice', { role: t(`role_${policy.userRole}`) }),
      });
      return;
    }
    await handleSavePolicy({
      enforcementMode: policy.enforcementMode,
      approvedAgents: policy.approvedAgents,
      approvedProviders: policy.approvedProviders,
      approvedModels: policy.approvedModels,
      approvedExtensions: policy.approvedExtensions,
      requireSignedExtensions:
        field === 'requireSignedExtensions' ? !policy.requireSignedExtensions : policy.requireSignedExtensions,
      allowLocalBreakGlass:
        field === 'allowLocalBreakGlass' ? !policy.allowLocalBreakGlass : policy.allowLocalBreakGlass,
    });
  }

  async function handleSavePolicy(req: {
    enforcementMode: string;
    approvedAgents: string[];
    approvedProviders: string[];
    approvedModels: string[];
    approvedExtensions: string[];
    requireSignedExtensions: boolean;
    allowLocalBreakGlass: boolean;
  }) {
    setSaving(true);
    setFeedback(null);
    try {
      await updateOrgPolicy(req);
      setFeedback({ tone: 'success', text: t('policySavedNotice') });
    } catch (caught) {
      const code = caught instanceof NexusError ? caught.code : 'ipc.unavailable';
      setFeedback({ tone: 'error', text: t(`errors:${code}`) });
    } finally {
      setSaving(false);
    }
  }

  async function handleBreakGlassToggle() {
    setFeedback(null);
    const nextEnabled = !policy.breakGlassActive;
    const reason = breakGlassReason.trim() || 'Emergency local developer recovery override';
    try {
      await triggerEmergencyBreakGlass(nextEnabled, reason);
      if (nextEnabled) {
        setBreakGlassReason('');
      }
    } catch (caught) {
      const code = caught instanceof NexusError ? caught.code : 'ipc.unavailable';
      setFeedback({ tone: 'error', text: t(`errors:${code}`) });
    }
  }

  async function handleProvisionTemplate(tpl: WorkspaceTemplateView) {
    setFeedback(null);
    try {
      const slug = tpl.id.replace(/^tpl_/, '');
      await createWorkspace({
        name: tpl.name,
        path: `~/NexusWorkspaces/${slug}`,
        primaryAgent: (tpl.primaryAgent as AgentId) || 'claude',
        fallbackAgent: (tpl.fallbackAgent as AgentId) || 'codex',
      });
      setFeedback({
        tone: 'success',
        text: t('templateAppliedNotice', { name: tpl.name }),
      });
    } catch (caught) {
      const code = caught instanceof NexusError ? caught.code : 'ipc.unavailable';
      setFeedback({ tone: 'error', text: t(`errors:${code}`) });
    }
  }

  return (
    <div className="governance-panel" data-testid="governance-control-panel">
      {/* 1. Organization Identity, Cryptographic Signature & RBAC Switcher */}
      <div className="sheet settings-sheet">
        <div className="sheet-body form-stack">
          <div className="governance-header-row">
            <div className="governance-org-meta">
              <span className="kicker">{t('orgBadgeLabel')}</span>
              <h3 className="governance-org-title">
                <ShieldCheck size={18} aria-hidden="true" />
                <span>{policy.orgName}</span>
                <span className="mono governance-org-id">{`(${policy.orgId})`}</span>
              </h3>
              <div className="governance-crypto-row">
                <span
                  className={`seal ${policy.signatureVerified ? '' : 'is-warning'}`}
                  data-s={policy.signatureVerified ? 'ready' : 'attention'}
                >
                  <KeyRound size={11} aria-hidden="true" />
                  <span>{t(policy.signatureVerified ? 'sigVerifiedBadge' : 'sigTamperedBadge')}</span>
                </span>
                <span className="mono governance-digest">
                  {`${t('digestPrefix')} ${policy.policyDigestSha256.slice(0, 26)}...`}
                </span>
              </div>
            </div>

            <div className="governance-rbac-box">
              <label className="setting-label" htmlFor="gov-role-select">
                <Users size={13} aria-hidden="true" />
                <span>{t('roleSelectorLabel')}</span>
              </label>
              <div className="governance-role-pills" id="gov-role-select" role="group" aria-label={t('roleSelectorLabel')}>
                {ROLES.map(role => (
                  <button
                    key={role}
                    type="button"
                    className={`button ${policy.userRole === role ? 'button-primary' : ''}`}
                    data-testid={`gov-role-${role}`}
                    onClick={() => void handleRoleChange(role)}
                  >
                    {t(`role_${role}`)}
                  </button>
                ))}
              </div>
              <p className="field-help">
                {t(policy.canMutatePolicy ? 'rolePermissionMutate' : 'rolePermissionReadOnly')}
              </p>
            </div>
          </div>

          {feedback && <Notice tone={feedback.tone}>{feedback.text}</Notice>}

          {/* Compliance Score & Posture Matrix */}
          <div className="governance-compliance-banner" data-status={compliance.overallStatus}>
            <div className="governance-score-ring">
              <span className="mono governance-score-number">{`${compliance.complianceScore}%`}</span>
              <span className="kicker">{t('complianceScoreTitle')}</span>
            </div>
            <div className="governance-score-details">
              <div className="governance-status-line">
                <span className="seal" data-s={compliance.overallStatus === 'compliant' ? 'ready' : 'attention'}>
                  {t(`complianceStatus_${compliance.overallStatus}`)}
                </span>
                <span className="field-help">
                  {t('checksTotalLabel', {
                    compliant: compliance.compliantCount,
                    total: compliance.totalChecks,
                  })}
                </span>
              </div>
              <div className="governance-checks-list">
                {compliance.checks.map(chk => (
                  <div key={chk.domain} className="governance-check-item">
                    <div className="governance-check-head">
                      <span className="governance-check-domain">{chk.domain}</span>
                      <span
                        className="seal"
                        data-s={chk.status === 'compliant' ? 'ready' : chk.status === 'advisory_drift' ? 'attention' : 'blocked'}
                      >
                        {t(`statusPill_${chk.status}`)}
                      </span>
                    </div>
                    <p className="field-help">{`${chk.subject} — ${chk.reason}`}</p>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* 2. Cryptographic Allowlists & Enforcement Mode */}
      <div className="sheet settings-sheet">
        <div className="sheet-body form-stack">
          <div className="governance-section-head">
            <h4>
              <Lock size={15} aria-hidden="true" />
              <span>{t('allowlistsTitle')}</span>
            </h4>
            <p className="field-help">{t('allowlistsSubtitle')}</p>
          </div>

          {!policy.canMutatePolicy && (
            <Notice>{t('rbacLockedNotice', { role: t(`role_${policy.userRole}`) })}</Notice>
          )}

          <div className="governance-control-block">
            <span className="setting-label">{t('enforcementModeLabel')}</span>
            <div className="governance-role-pills" role="group" aria-label={t('enforcementModeLabel')}>
              {MODES.map(m => (
                <button
                  key={m}
                  type="button"
                  disabled={!policy.canMutatePolicy || saving}
                  className={`button ${policy.enforcementMode === m ? 'button-primary' : ''}`}
                  data-testid={`gov-mode-${m}`}
                  onClick={() => void handleModeChange(m)}
                >
                  {t(`mode_${m}`)}
                </button>
              ))}
            </div>
          </div>

          <div className="governance-allowlist-grid">
            <div className="governance-allowlist-col">
              <span className="setting-label">{t('approvedAgentsLabel')}</span>
              <div className="governance-chip-group">
                {ALL_AGENTS.map(agent => {
                  const active = policy.approvedAgents.includes(agent);
                  return (
                    <button
                      key={agent}
                      type="button"
                      disabled={!policy.canMutatePolicy || saving}
                      className={`button ${active ? 'button-primary' : ''}`}
                      data-testid={`gov-agent-${agent}`}
                      onClick={() => void handleToggleAgent(agent)}
                    >
                      <CheckCircle2 size={12} aria-hidden="true" />
                      <span className="mono">{agent}</span>
                    </button>
                  );
                })}
              </div>
            </div>

            <div className="governance-allowlist-col">
              <span className="setting-label">{t('approvedProvidersLabel')}</span>
              <div className="governance-chip-group">
                {ALL_PROVIDERS.map(provider => {
                  const active = policy.approvedProviders.includes(provider);
                  return (
                    <button
                      key={provider}
                      type="button"
                      disabled={!policy.canMutatePolicy || saving}
                      className={`button ${active ? 'button-primary' : ''}`}
                      data-testid={`gov-provider-${provider}`}
                      onClick={() => void handleToggleProvider(provider)}
                    >
                      <CheckCircle2 size={12} aria-hidden="true" />
                      <span className="mono">{provider}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          </div>

          <div className="governance-toggles-row">
            <label className="governance-checkbox-row">
              <input
                type="checkbox"
                checked={policy.requireSignedExtensions}
                disabled={!policy.canMutatePolicy || saving}
                onChange={() => void handleToggleFlag('requireSignedExtensions')}
              />
              <span>{t('requireSignedExtensionsLabel')}</span>
            </label>
            <label className="governance-checkbox-row">
              <input
                type="checkbox"
                checked={policy.allowLocalBreakGlass}
                disabled={!policy.canMutatePolicy || saving}
                onChange={() => void handleToggleFlag('allowLocalBreakGlass')}
              />
              <span>{t('allowLocalBreakGlassLabel')}</span>
            </label>
          </div>

          <div className="governance-precedence-box">
            <span className="kicker">{t('precedenceTitle')}</span>
            <ol className="governance-precedence-list">
              {policy.precedenceChain.map(step => (
                <li key={step} className="mono">
                  {step}
                </li>
              ))}
            </ol>
          </div>
        </div>
      </div>

      {/* 3. Standardized Organization Workspace Templates */}
      <div className="sheet settings-sheet">
        <div className="sheet-body form-stack">
          <div className="governance-section-head">
            <h4>
              <Layers size={15} aria-hidden="true" />
              <span>{t('templatesTitle')}</span>
            </h4>
            <p className="field-help">{t('templatesSubtitle')}</p>
          </div>

          <div className="governance-templates-grid">
            {templates.map(tpl => (
              <div key={tpl.id} className="governance-template-card" data-testid={`gov-template-${tpl.id}`}>
                <div className="governance-template-head">
                  <h5>{tpl.name}</h5>
                  <span className="mono kicker">{tpl.id}</span>
                </div>
                <p className="field-help">{tpl.description}</p>
                <div className="governance-template-badges">
                  <span className="seal" data-s="ready">
                    {t('templatePrimaryAgent', { agent: tpl.primaryAgent })}
                  </span>
                  {tpl.fallbackAgent && (
                    <span className="seal" data-s="unknown">
                      {t('templateFallbackAgent', { agent: tpl.fallbackAgent })}
                    </span>
                  )}
                  <span className="mono governance-template-model">
                    {t('templateModel', { model: tpl.recommendedModel })}
                  </span>
                </div>
                <div className="governance-template-exts">
                  <span className="kicker">{t('templateExtensions')}</span>
                  <div className="governance-chip-group">
                    {tpl.requiredExtensions.map(ext => (
                      <span key={ext} className="mono governance-ext-pill">
                        {ext}
                      </span>
                    ))}
                  </div>
                </div>
                <pre className="plan-diff mono" tabIndex={0} aria-label={tpl.name}>
                  {tpl.instructionTemplate}
                </pre>
                <div className="plan-actions">
                  <button
                    type="button"
                    className="button button-primary"
                    data-testid={`gov-provision-${tpl.id}`}
                    onClick={() => void handleProvisionTemplate(tpl)}
                  >
                    <Sparkles size={13} aria-hidden="true" />
                    <span>{t('applyTemplateButton')}</span>
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* 4. Scoped Health Visibility & Zero-Knowledge Privacy Proof */}
      <div className="sheet settings-sheet">
        <div className="sheet-body form-stack">
          <div className="governance-section-head">
            <h4>
              <Shield size={15} aria-hidden="true" />
              <span>{t('visibilityTitle')}</span>
            </h4>
            <p className="field-help">{t('visibilitySubtitle')}</p>
          </div>

          <div className="governance-visibility-grid">
            <div className="governance-visibility-card is-shared">
              <h5>
                <Eye size={14} aria-hidden="true" />
                <span>{t('sharedColumnTitle')}</span>
              </h5>
              <ul>
                {visibilityBoundary.sharedFields.map(item => (
                  <li key={item}>{item}</li>
                ))}
              </ul>
            </div>

            <div className="governance-visibility-card is-private">
              <h5>
                <EyeOff size={14} aria-hidden="true" />
                <span>{t('neverSharedColumnTitle')}</span>
              </h5>
              <ul>
                {visibilityBoundary.neverSharedFields.map(item => (
                  <li key={item}>{item}</li>
                ))}
              </ul>
            </div>
          </div>
          <p className="mono field-help">{visibilityBoundary.cryptoProof}</p>
        </div>
      </div>

      {/* 5. Emergency Break-Glass Override & Immutable Audit Log */}
      <div className="sheet settings-sheet">
        <div className="sheet-body form-stack">
          <div className="governance-section-head">
            <h4>
              <ShieldAlert size={15} aria-hidden="true" />
              <span>{t('breakGlassTitle')}</span>
            </h4>
            <p className="field-help">{t('breakGlassSubtitle')}</p>
          </div>

          <div
            className={`governance-breakglass-box ${policy.breakGlassActive ? 'is-active' : ''}`}
            data-testid="gov-breakglass-box"
          >
            <div className="governance-breakglass-status">
              <AlertTriangle size={16} aria-hidden="true" />
              <span>
                {t(policy.breakGlassActive ? 'breakGlassStatusActive' : 'breakGlassStatusStandby')}
              </span>
            </div>
            <div className="governance-breakglass-controls">
              {!policy.breakGlassActive && (
                <input
                  type="text"
                  className="text-input"
                  value={breakGlassReason}
                  onChange={e => setBreakGlassReason(e.target.value)}
                  placeholder={t('breakGlassReasonPlaceholder')}
                  aria-label={t('breakGlassReasonPlaceholder')}
                />
              )}
              <button
                type="button"
                className={`button ${policy.breakGlassActive ? '' : 'button-danger'}`}
                data-testid="gov-breakglass-toggle"
                onClick={() => void handleBreakGlassToggle()}
              >
                {t(policy.breakGlassActive ? 'deactivateBreakGlassButton' : 'activateBreakGlassButton')}
              </button>
            </div>
          </div>

          <div className="governance-audit-section">
            <h5>
              <FileCheck2 size={14} aria-hidden="true" />
              <span>{t('auditLogTitle')}</span>
            </h5>
            <div className="governance-audit-list" data-testid="gov-audit-log">
              {auditLog.slice(0, 8).map(entry => (
                <div key={entry.id} className="governance-audit-row">
                  <span className="mono seal" data-s="ready">
                    {entry.actorRole}
                  </span>
                  <span className="mono governance-audit-action">{entry.action}</span>
                  <span className="governance-audit-details">{entry.details}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
