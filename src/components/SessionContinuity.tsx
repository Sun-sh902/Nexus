import { useState } from 'react';
import { Terminal, Trash2, Clock, MessageSquare, Zap } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNexus } from '../app/nexus-context';
import type { AgentId, SessionSummary } from '../app/backend';
import { Notice } from './ui';

const AGENT_NAMES: Record<AgentId, string> = {
  claude: 'Claude Code',
  codex: 'OpenAI Codex',
  gemini: 'Gemini CLI',
  opencode: 'OpenCode',
};

interface SessionContinuityProps {
  workspaceId?: string;
  agentId?: AgentId;
  compact?: boolean;
}

export function SessionContinuity({ workspaceId, agentId, compact = false }: SessionContinuityProps) {
  const { t } = useTranslation(['sessions', 'common', 'errors']);
  const { sessions, resumeSession, deleteSession } = useNexus();
  const [resumingId, setResumingId] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ tone: 'success' | 'attention'; message: string } | null>(null);

  const filteredSessions = sessions.filter(s => {
    if (workspaceId && s.workspaceId !== workspaceId) return false;
    if (agentId && s.agentId !== agentId) return false;
    return true;
  });

  const handleResume = async (session: SessionSummary) => {
    setResumingId(session.id);
    setNotice(null);
    try {
      await resumeSession(session.id);
      setNotice({
        tone: 'success',
        message: `${t('resumeSuccess')}: ${session.title}`,
      });
      setTimeout(() => setNotice(null), 4000);
    } catch (err: unknown) {
      const code = err && typeof err === 'object' && 'code' in err ? String((err as { code: string }).code) : 'lifecycle.launch_failed';
      setNotice({
        tone: 'attention',
        message: t(`errors:${code}`, { defaultValue: t('resumeFailed') }),
      });
    } finally {
      setResumingId(null);
    }
  };

  const handleDelete = async (session: SessionSummary) => {
    if (!window.confirm(t('deleteConfirm'))) return;
    try {
      await deleteSession(session.id);
    } catch {
      // Non-fatal
    }
  };

  if (filteredSessions.length === 0) {
    return (
      <div className="session-continuity-empty">
        <p className="kicker">{t('kicker')}</p>
        <p className="empty-desc">{t('noSessionsHelp')}</p>
      </div>
    );
  }

  return (
    <div className={`session-continuity ${compact ? 'is-compact' : ''}`}>
      {notice && (
        <Notice tone={notice.tone}>
          {notice.message}
        </Notice>
      )}

      <div className="session-list" role="list" aria-label={t('title')}>
        {filteredSessions.map(session => {
          const isResuming = resumingId === session.id;
          return (
            <div key={session.id} className="session-card" role="listitem">
              <div className="session-card-header">
                <span className="session-agent-badge mono">
                  {AGENT_NAMES[session.agentId] || session.agentId}
                </span>
                <span className="session-time mono" title={session.lastActiveAt}>
                  <Clock size={11} aria-hidden="true" />
                  {session.lastActiveAt.slice(0, 10)}
                </span>
              </div>

              <h5 className="session-title">{session.title}</h5>

              <div className="session-meta">
                <span className="session-metric" title={t('tokensDetail', { input: session.inputTokens.toLocaleString(), output: session.outputTokens.toLocaleString() })}>
                  <Zap size={11} aria-hidden="true" />
                  {t('tokens', { count: session.totalTokens.toLocaleString() })}
                </span>
                <span className="session-metric">
                  <MessageSquare size={11} aria-hidden="true" />
                  {t('messages', { count: session.messageCount })}
                </span>
                <span className="session-cost-badge mono" title={session.costProvenance}>
                  {t('estimatedCost', { cost: session.estimatedCostUsd })}
                </span>
              </div>

              <div className="session-actions">
                <button
                  type="button"
                  className="button button-small button-primary"
                  disabled={isResuming || !session.canResume}
                  onClick={() => void handleResume(session)}
                  aria-label={`${t('resume')} ${session.title}`}
                >
                  <Terminal size={12} aria-hidden="true" />
                  <span>{isResuming ? t('resuming') : t('resume')}</span>
                </button>

                <button
                  type="button"
                  className="button button-small button-ghost"
                  onClick={() => void handleDelete(session)}
                  title={t('delete')}
                  aria-label={t('delete')}
                >
                  <Trash2 size={12} aria-hidden="true" />
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
