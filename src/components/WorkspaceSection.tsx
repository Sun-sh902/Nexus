import { useState } from 'react';
import { Folder, GitBranch, Terminal, Trash2, Plus, FileCode2, Clock, BarChart3, Sparkles } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNexus } from '../app/nexus-context';
import type { AgentId, DiscoveredWorkspaceCandidate } from '../app/backend';
import { Section, Status, Notice } from './ui';
import { SessionContinuity } from './SessionContinuity';
import { UsageInsightsPanel } from './UsageInsightsPanel';

const AGENTS: { id: AgentId; name: string }[] = [
  { id: 'claude', name: 'Claude Code' },
  { id: 'codex', name: 'OpenAI Codex' },
  { id: 'gemini', name: 'Gemini CLI' },
  { id: 'opencode', name: 'OpenCode' },
];

export function WorkspaceSection() {
  const { t } = useTranslation(['workspaces', 'common', 'errors']);
  const { workspaces, sessions, discoveredWorkspaces, discoverLocalWorkspaces, createWorkspace, deleteWorkspace, launchWorkspace } = useNexus();
  const [adding, setAdding] = useState(false);
  const [showUsage, setShowUsage] = useState(false);
  const [showGlobalSessions, setShowGlobalSessions] = useState(false);
  const [showDiscover, setShowDiscover] = useState(false);
  const [importingPath, setImportingPath] = useState<string | null>(null);
  const [expandedWsId, setExpandedWsId] = useState<string | null>(null);
  const [path, setPath] = useState('');
  const [name, setName] = useState('');
  const [primaryAgent, setPrimaryAgent] = useState<AgentId>('claude');
  const [fallbackAgent, setFallbackAgent] = useState<AgentId | ''>('');
  const [submitting, setSubmitting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [launchMsg, setLaunchMsg] = useState<string | null>(null);
  const [launchingId, setLaunchingId] = useState<string | null>(null);

  const handleImportCandidate = async (cand: DiscoveredWorkspaceCandidate) => {
    setErrorMsg(null);
    setImportingPath(cand.path);
    try {
      await createWorkspace({
        name: cand.name,
        path: cand.path,
        primaryAgent: cand.primaryAgent,
      });
    } catch (err: unknown) {
      const code = err && typeof err === 'object' && 'code' in err ? String((err as { code: string }).code) : 'workspace.invalid_path';
      setErrorMsg(t(`errors:${code}`, { defaultValue: t('errors:app.task_failed') }));
    } finally {
      setImportingPath(null);
    }
  };

  const handleImportAllCandidates = async () => {
    setErrorMsg(null);
    for (const cand of discoveredWorkspaces) {
      setImportingPath(cand.path);
      try {
        await createWorkspace({
          name: cand.name,
          path: cand.path,
          primaryAgent: cand.primaryAgent,
        });
      } catch {
        // continue importing remaining
      }
    }
    setImportingPath(null);
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMsg(null);
    if (!path.trim()) {
      setErrorMsg(t('errors:workspace.invalid_path'));
      return;
    }
    setSubmitting(true);
    try {
      await createWorkspace({
        name: name.trim() || undefined,
        path: path.trim(),
        primaryAgent,
        fallbackAgent: fallbackAgent || undefined,
      });
      setAdding(false);
      setPath('');
      setName('');
    } catch (err: unknown) {
      const code = err && typeof err === 'object' && 'code' in err ? String((err as { code: string }).code) : 'workspace.invalid_path';
      setErrorMsg(t(`errors:${code}`, { defaultValue: t('errors:app.task_failed') }));
    } finally {
      setSubmitting(false);
    }
  };

  const handleLaunch = async (id: string, wsName: string) => {
    setLaunchingId(id);
    setLaunchMsg(null);
    try {
      await launchWorkspace(id);
      setLaunchMsg(`${t('launchSuccess')}: ${wsName}`);
      setTimeout(() => setLaunchMsg(null), 4000);
    } catch (err: unknown) {
      const code = err && typeof err === 'object' && 'code' in err ? String((err as { code: string }).code) : 'lifecycle.launch_failed';
      setErrorMsg(t(`errors:${code}`, { defaultValue: t('errors:app.task_failed') }));
    } finally {
      setLaunchingId(null);
    }
  };

  const handleDelete = async (id: string, wsName: string) => {
    if (!window.confirm(t('deleteConfirm', { name: wsName }))) return;
    try {
      await deleteWorkspace(id);
    } catch (err: unknown) {
      const code = err && typeof err === 'object' && 'code' in err ? String((err as { code: string }).code) : 'app.task_failed';
      setErrorMsg(t(`errors:${code}`, { defaultValue: t('errors:app.task_failed') }));
    }
  };

  return (
    <Section
      title={t('title')}
      className="home-workspaces"
      aside={
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap' }}>
          <button
            type="button"
            className="button button-small"
            onClick={() => {
              const next = !showDiscover;
              setShowDiscover(next);
              if (next) void discoverLocalWorkspaces();
            }}
            aria-pressed={showDiscover}
          >
            <Sparkles size={13} aria-hidden="true" />
            <span>{showDiscover ? t('hideDiscover') : t('discoverToggle', { count: discoveredWorkspaces.length })}</span>
          </button>
          <button
            type="button"
            className="button button-small"
            onClick={() => setShowGlobalSessions(prev => !prev)}
            aria-pressed={showGlobalSessions}
          >
            <Clock size={13} aria-hidden="true" />
            <span>{showGlobalSessions ? t('hideGlobalSessions') : t('globalSessionsToggle', { count: sessions.length })}</span>
          </button>
          <button
            type="button"
            className="button button-small"
            onClick={() => setShowUsage(prev => !prev)}
            aria-pressed={showUsage}
          >
            <BarChart3 size={13} aria-hidden="true" />
            <span>{showUsage ? t('hideUsage') : t('usageToggle')}</span>
          </button>
          <button
            type="button"
            className="button button-small"
            onClick={() => { setAdding(prev => !prev); setErrorMsg(null); }}
          >
            <Plus size={13} aria-hidden="true" />
            <span>{adding ? t('cancel') : t('addWorkspace')}</span>
          </button>
        </div>
      }
    >
      {errorMsg && <Notice tone="attention">{errorMsg}</Notice>}
      {launchMsg && <Notice tone="success">{launchMsg}</Notice>}
      {showUsage && <UsageInsightsPanel />}
      {showGlobalSessions && (
        <div className="sheet" style={{ padding: '12px', marginBottom: '12px' }}>
          <SessionContinuity />
        </div>
      )}
      {showDiscover && (
        <div className="sheet" style={{ padding: '14px', marginBottom: '12px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px', flexWrap: 'wrap', marginBottom: '10px' }}>
            <div>
              <h4 style={{ margin: 0 }}>{t('discoverTitle')}</h4>
              <p className="field-help" style={{ margin: '2px 0 0' }}>{t('discoverSubtitle')}</p>
            </div>
            {discoveredWorkspaces.length > 1 && (
              <button
                type="button"
                className="button button-small button-primary"
                disabled={importingPath !== null}
                onClick={() => void handleImportAllCandidates()}
              >
                <span>{t('discoverImportAll', { count: discoveredWorkspaces.length })}</span>
              </button>
            )}
          </div>
          {discoveredWorkspaces.length === 0 ? (
            <p className="field-help" style={{ margin: 0 }}>{t('discoverEmpty')}</p>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {discoveredWorkspaces.map(cand => {
                const agentLabel = AGENTS.find(a => a.id === cand.primaryAgent)?.name ?? cand.primaryAgent;
                return (
                  <div
                    key={cand.path}
                    style={{
                      display: 'flex',
                      flexWrap: 'wrap',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      gap: '8px',
                      padding: '8px 10px',
                      borderRadius: '6px',
                      border: '1px solid var(--border-subtle, rgba(128,128,128,0.2))',
                    }}
                  >
                    <div style={{ minWidth: 0, flex: '1 1 200px' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexWrap: 'wrap' }}>
                        <Folder size={14} aria-hidden="true" />
                        <strong lang="en-US">{cand.name}</strong>
                        <span className="chip">{agentLabel}</span>
                        {cand.gitBranch && (
                          <span className="workspace-branch mono">
                            <GitBranch size={11} aria-hidden="true" />
                            <span>{cand.gitBranch}</span>
                          </span>
                        )}
                        {cand.sessionCount > 0 && (
                          <span className="chip mono">{t('discoverSessionsBadge', { count: cand.sessionCount })}</span>
                        )}
                        {cand.instructionFiles.map(f => (
                          <span key={f} className="instruction-tag mono">{f}</span>
                        ))}
                      </div>
                      <div className="mono" style={{ fontSize: '0.76rem', opacity: 0.75, overflowWrap: 'anywhere', marginTop: '2px' }}>
                        {cand.displayPath}
                      </div>
                    </div>
                    <button
                      type="button"
                      className="button button-small"
                      disabled={importingPath !== null}
                      onClick={() => void handleImportCandidate(cand)}
                    >
                      <span>{t('discoverImport')}</span>
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {adding && (
        <form onSubmit={handleCreate} className="add-workspace-card">
          <h4>{t('addWorkspace')}</h4>
          <div className="form-grid">
            <label>
              <span>{t('pathLabel')}</span>
              <input
                type="text"
                className="search-field"
                placeholder={t('pathPlaceholder')}
                value={path}
                onChange={e => setPath(e.target.value)}
                required
              />
            </label>
            <label>
              <span>{t('nameLabel')}</span>
              <input
                type="text"
                className="search-field"
                placeholder={t('namePlaceholder')}
                value={name}
                onChange={e => setName(e.target.value)}
              />
            </label>
            <label>
              <span>{t('primaryAgentLabel')}</span>
              <select
                className="search-field"
                value={primaryAgent}
                onChange={e => setPrimaryAgent(e.target.value as AgentId)}
              >
                {AGENTS.map(a => (
                  <option key={a.id} value={a.id}>{a.name}</option>
                ))}
              </select>
            </label>
            <label>
              <span>{t('fallbackAgentLabel')}</span>
              <select
                className="search-field"
                value={fallbackAgent}
                onChange={e => setFallbackAgent(e.target.value as AgentId | '')}
              >
                <option value="">{t('none')}</option>
                {AGENTS.filter(a => a.id !== primaryAgent).map(a => (
                  <option key={a.id} value={a.id}>{a.name}</option>
                ))}
              </select>
            </label>
          </div>
          <div className="row-actions" style={{ marginTop: '14px' }}>
            <button type="submit" className="button button-primary button-small" disabled={submitting}>
              <span>{submitting ? t('saving') : t('save')}</span>
            </button>
            <button
              type="button"
              className="button button-small"
              onClick={() => { setAdding(false); setErrorMsg(null); }}
            >
              <span>{t('cancel')}</span>
            </button>
          </div>
        </form>
      )}

      {workspaces.length === 0 && !adding ? (
        <Notice tone="info">
          <strong>{t('emptyTitle')}</strong>
          <p>{t('emptyHelp')}</p>
        </Notice>
      ) : (
        <div className="workspace-grid">
          {workspaces.map(ws => {
            const activeAgentName = AGENTS.find(a => a.id === ws.activeAgent)?.name ?? ws.activeAgent;
            const primaryAgentName = AGENTS.find(a => a.id === ws.primaryAgent)?.name ?? ws.primaryAgent;
            const isUsingFallback = ws.activeAgent !== ws.primaryAgent;

            return (
              <div key={ws.id} className="workspace-card">
                <div>
                  <div className="workspace-card-head">
                    <div className="workspace-title-group">
                      <h3>
                        <Folder size={16} strokeWidth={1.8} aria-hidden="true" />
                        <span>{ws.name}</span>
                      </h3>
                      <div className="workspace-path mono">{ws.displayPath}</div>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      {ws.isGitRepo && ws.gitBranch && (
                        <span className="workspace-branch mono" title={t('gitBranch')}>
                          <GitBranch size={11} aria-hidden="true" />
                          <span>{ws.gitBranch}</span>
                        </span>
                      )}
                      <Status state={ws.readiness} />
                    </div>
                  </div>

                  <div className="workspace-meta-row">
                    <span className="chip">
                      <strong>{t('activeAgentLabel')}</strong>
                      <span>{` ${activeAgentName}`}</span>
                      {isUsingFallback && <span>{` ${t('fallbackBadge')}`}</span>}
                    </span>
                    {isUsingFallback && (
                      <span className="chip" style={{ opacity: 0.8 }}>
                        <span>{t('primaryAgentPrefix', { agent: primaryAgentName })}</span>
                      </span>
                    )}

                    {ws.instructions.length > 0 ? (
                      ws.instructions.map(inst => (
                        <span
                          key={inst.relativePath}
                          className="instruction-tag"
                          title={`${inst.relativePath} - ${t('lines', { count: inst.lineCount })}`}
                        >
                          <FileCode2 size={11} aria-hidden="true" />
                          <strong>{inst.fileName}</strong>
                          <span className="mono">{t('lineBadge', { count: inst.lineCount })}</span>
                        </span>
                      ))
                    ) : (
                      <span className="instruction-tag" style={{ opacity: 0.7 }}>
                        <span>{t('noInstructions')}</span>
                      </span>
                    )}
                  </div>
                </div>

                <div className="workspace-actions">
                  <button
                    type="button"
                    className="button button-small"
                    onClick={() => setExpandedWsId(prev => prev === ws.id ? null : ws.id)}
                    aria-expanded={expandedWsId === ws.id}
                  >
                    <Clock size={12} aria-hidden="true" />
                    <span>
                      {expandedWsId === ws.id
                        ? t('hideSessions')
                        : t('sessionsToggle', { count: sessions.filter(s => s.workspaceId === ws.id).length })}
                    </span>
                  </button>
                  <button
                    type="button"
                    className="button button-small button-primary"
                    disabled={launchingId === ws.id || ws.readiness === 'blocked'}
                    onClick={() => handleLaunch(ws.id, ws.name)}
                    title={ws.readiness === 'blocked' ? ws.readinessReasons.join(', ') : t('launchTerminal')}
                  >
                    <Terminal size={12} aria-hidden="true" />
                    <span>{launchingId === ws.id ? t('launching') : t('launchTerminalWith', { agent: activeAgentName })}</span>
                  </button>
                  <button
                    type="button"
                    className="icon-button"
                    onClick={() => handleDelete(ws.id, ws.name)}
                    aria-label={t('delete')}
                    title={t('delete')}
                  >
                    <Trash2 size={14} aria-hidden="true" />
                  </button>
                </div>
                {expandedWsId === ws.id && (
                  <div style={{ marginTop: '12px', paddingTop: '12px', borderTop: '1px solid var(--border)' }}>
                    <SessionContinuity workspaceId={ws.id} compact />
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </Section>
  );
}
