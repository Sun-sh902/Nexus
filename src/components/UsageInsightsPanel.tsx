import { ShieldCheck, Cpu, DollarSign, Activity } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNexus } from '../app/nexus-context';
import type { AgentId } from '../app/backend';

const AGENT_NAMES: Record<AgentId, string> = {
  claude: 'Claude Code',
  codex: 'OpenAI Codex',
  gemini: 'Gemini CLI',
  opencode: 'OpenCode',
};

interface UsageInsightsPanelProps {
  workspaceId?: string;
  agentId?: AgentId;
}

export function UsageInsightsPanel({ workspaceId, agentId }: UsageInsightsPanelProps) {
  const { t } = useTranslation(['usage', 'common']);
  const { usageInsights } = useNexus();

  if (!usageInsights) {
    return null;
  }

  const byAgentFiltered = usageInsights.byAgent.filter(a => !agentId || a.agentId === agentId);
  const byWorkspaceFiltered = usageInsights.byWorkspace.filter(w => !workspaceId || w.workspaceId === workspaceId);

  return (
    <div className="usage-insights-panel">
      <div className="usage-stat-grid">
        <div className="usage-stat-card">
          <div className="stat-icon-wrapper" aria-hidden="true">
            <Cpu size={16} />
          </div>
          <div className="stat-content">
            <span className="stat-label">{t('totalTokens')}</span>
            <strong className="stat-value mono">
              {usageInsights.totalTokens.toLocaleString()}
            </strong>
            <span className="stat-sub mono">
              {t('tokensInOut', { input: usageInsights.totalInputTokens.toLocaleString(), output: usageInsights.totalOutputTokens.toLocaleString() })}
            </span>
          </div>
        </div>

        <div className="usage-stat-card">
          <div className="stat-icon-wrapper" aria-hidden="true">
            <DollarSign size={16} />
          </div>
          <div className="stat-content">
            <span className="stat-label">{t('estimatedCost')}</span>
            <strong className="stat-value mono">
              {usageInsights.totalEstimatedCostUsd}
            </strong>
            <span className="stat-sub">
              {t('estimated')}
            </span>
          </div>
        </div>

        <div className="usage-stat-card">
          <div className="stat-icon-wrapper" aria-hidden="true">
            <Activity size={16} />
          </div>
          <div className="stat-content">
            <span className="stat-label">{t('activeSessions')}</span>
            <strong className="stat-value mono">
              {usageInsights.totalSessionsCount}
            </strong>
            <span className="stat-sub">
              {t('sessionsCount', { count: usageInsights.totalSessionsCount })}
            </span>
          </div>
        </div>
      </div>

      {byAgentFiltered.length > 0 && (
        <div className="usage-breakdown-section">
          <h5>{t('byAgent')}</h5>
          <div className="breakdown-bars">
            {byAgentFiltered.map(item => (
              <div key={item.agentId} className="breakdown-row">
                <div className="breakdown-label">
                  <span className="agent-title">{AGENT_NAMES[item.agentId] || item.agentId}</span>
                  <span className="agent-detail mono">
                    {t('agentCostDetail', { tokens: t('tokenCount', { count: item.totalTokens.toLocaleString() }), cost: item.estimatedCostUsd })}
                  </span>
                </div>
                <div className="progress-bar-bg" aria-hidden="true">
                  <div
                    className="progress-bar-fill"
                    style={{ width: `${Math.max(4, Math.min(100, item.percentageOfTotal))}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {byWorkspaceFiltered.length > 0 && !workspaceId && (
        <div className="usage-breakdown-section">
          <h5>{t('byWorkspace')}</h5>
          <div className="breakdown-bars">
            {byWorkspaceFiltered.map(ws => (
              <div key={ws.workspaceId} className="breakdown-row">
                <div className="breakdown-label">
                  <span className="agent-title">{ws.workspaceName}</span>
                  <span className="agent-detail mono">
                    {t('agentCostDetail', { tokens: t('tokenCount', { count: ws.totalTokens.toLocaleString() }), cost: ws.estimatedCostUsd })}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {usageInsights.byModel.length > 0 && (
        <div className="usage-breakdown-section">
          <h5>{t('byModel')}</h5>
          <div className="model-table-wrapper">
            <table className="model-usage-table">
              <thead>
                <tr>
                  <th>{t('byModel')}</th>
                  <th className="num-col">{t('totalTokens')}</th>
                  <th className="num-col">{t('estimatedCost')}</th>
                </tr>
              </thead>
              <tbody>
                {usageInsights.byModel.map(model => (
                  <tr key={model.modelIdentifier}>
                    <td className="mono">{model.modelIdentifier}</td>
                    <td className="num-col mono">{model.totalTokens.toLocaleString()}</td>
                    <td className="num-col mono">{model.estimatedCostUsd}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <div className="usage-provenance-box">
        <div className="provenance-header">
          <ShieldCheck size={14} aria-hidden="true" />
          <h6>{t('provenanceTitle')}</h6>
        </div>
        <ul className="provenance-list">
          {usageInsights.provenanceNotes.map(noteKey => (
            <li key={noteKey}>{t(noteKey)}</li>
          ))}
        </ul>
      </div>
    </div>
  );
}
