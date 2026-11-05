import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useNexus } from '../app/nexus-context';
import { useActionFlow } from '../app/action-flow';
import type { AgentId, AiDoctorConsultation } from '../app/backend';
import { HealthRadar } from './HealthRadar';

interface AiDoctorPanelProps {
  agentId: AgentId;
}

export function AiDoctorPanel({ agentId }: AiDoctorPanelProps) {
  const { t } = useTranslation();
  const { consultAiDoctor } = useNexus();
  const flow = useActionFlow();

  const [consultation, setConsultation] = useState<AiDoctorConsultation | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleConsult = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await consultAiDoctor(agentId);
      setConsultation(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [agentId, consultAiDoctor]);

  const getSeverityBadge = (severity: string) => {
    switch (severity) {
      case 'healthy':
        return {
          bg: 'var(--ready-soft)',
          border: 'var(--ready)',
          text: 'var(--ready-ink)',
          label: t('aiDoctor:severityHealthy'),
        };
      case 'advisory':
        return {
          bg: 'var(--attention-soft)',
          border: 'var(--attention)',
          text: 'var(--attention-ink)',
          label: t('aiDoctor:severityAdvisory'),
        };
      case 'critical':
      default:
        return {
          bg: 'var(--blocked-soft)',
          border: 'var(--blocked)',
          text: 'var(--blocked-ink)',
          label: t('aiDoctor:severityCritical'),
        };
    }
  };

  return (
    <div className="rounded-lg border border-[var(--border)] bg-[var(--sheet)] shadow-[var(--shadow-sheet)] p-5 mb-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-4 mb-4 border-b border-[var(--border-subtle)]">
        <div>
          <div className="flex items-center gap-2">
            <span className="w-2.5 h-2.5 rounded-full bg-[var(--brand)] animate-pulse" />
            <h2 className="text-base font-semibold text-[var(--ink)]">
              {t('aiDoctor:title')}
            </h2>
          </div>
          <p className="text-xs text-[var(--ink-3)] mt-0.5">
            {t('aiDoctor:subtitle')}
          </p>
        </div>

        <button
          type="button"
          onClick={handleConsult}
          disabled={loading}
          className="inline-flex items-center justify-center px-3.5 py-1.5 rounded-md text-xs font-medium bg-[var(--brand)] text-[var(--on-brand)] hover:opacity-90 disabled:opacity-50 transition-all cursor-pointer focus:outline-none focus:ring-2 focus:ring-[var(--focus)]"
          aria-label={t('aiDoctor:consultButton')}
        >
          {loading ? t('aiDoctor:consulting') : t('aiDoctor:consultButton')}
        </button>
      </div>

      {/* Consultation Results */}
      {consultation && (
        <div className="space-y-5 animate-in fade-in duration-300">
          {/* Health Radar Pipeline */}
          <HealthRadar stages={consultation.radarStages} />

          {/* Diagnostic Summary & Grounded Confidence */}
          <div className="p-3.5 rounded-md bg-[var(--sunken)] border border-[var(--border)]">
            <div className="flex items-center justify-between gap-2 mb-2">
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold uppercase tracking-wider text-[var(--ink-3)]">
                  {t('aiDoctor:overallSeverity')}
                </span>
                {(() => {
                  const badge = getSeverityBadge(consultation.overallSeverity);
                  return (
                    <span
                      className="px-2 py-0.5 rounded text-xs font-medium border"
                      style={{ backgroundColor: badge.bg, borderColor: badge.border, color: badge.text }}
                    >
                      {badge.label}
                    </span>
                  );
                })()}
              </div>

              <div className="text-xs font-mono text-[var(--ink-2)]">
                {t('aiDoctor:confidenceLabel')}
                {t('aiDoctor:colon')}
                {Math.round(consultation.confidenceScore * 100)}
                {t('aiDoctor:percent')}
              </div>
            </div>

            <p className="text-sm leading-relaxed text-[var(--ink)]">
              {consultation.analysisSummary}
            </p>
          </div>

          {/* Ranked Probable Causes */}
          <div>
            <h3 className="text-xs font-semibold uppercase tracking-wider text-[var(--ink-3)] mb-2.5">
              {t('aiDoctor:rankedCausesTitle')}
            </h3>

            <div className="space-y-2">
              {consultation.rankedCauses.map((cause) => (
                <div
                  key={`${cause.rank}-${cause.category}`}
                  className="p-3 rounded-md border border-[var(--border)] bg-[var(--raised)] flex flex-col gap-1.5"
                >
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-bold text-[var(--brand)]">
                      {t('aiDoctor:rankLabel', { rank: cause.rank })}
                      {t('aiDoctor:colon')}
                      {cause.category}
                    </span>
                    <span className="text-[11px] px-1.5 py-0.5 rounded uppercase font-mono border border-[var(--border)] text-[var(--ink-3)]">
                      {cause.confidence}
                    </span>
                  </div>

                  <p className="text-xs font-medium text-[var(--ink)]">
                    {cause.cause}
                  </p>

                  <div className="text-[11px] font-mono text-[var(--ink-3)] bg-[var(--sunken)] p-2 rounded border border-[var(--border-subtle)] break-all">
                    {t('aiDoctor:evidenceLabel')}
                    {t('aiDoctor:colon')}
                    {cause.evidenceSnippet}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Verified Safe Repairs */}
          {consultation.recommendations.length > 0 && (
            <div>
              <h3 className="text-xs font-semibold uppercase tracking-wider text-[var(--ink-3)] mb-2.5">
                {t('aiDoctor:recommendationsTitle')}
              </h3>

              <div className="space-y-2.5">
                {consultation.recommendations.map((rec) => (
                  <div
                    key={rec.id}
                    className="p-3.5 rounded-md border border-[var(--ready)] bg-[var(--ready-soft)] flex flex-col sm:flex-row sm:items-center justify-between gap-3"
                  >
                    <div className="space-y-1">
                      <div className="text-xs font-bold text-[var(--ready-ink)]">
                        {rec.title}
                      </div>
                      <div className="text-xs text-[var(--ink)]">
                        {t('aiDoctor:rationaleLabel')}
                        {t('aiDoctor:colon')}
                        {rec.rationale}
                      </div>
                      <div className="text-[11px] font-mono text-[var(--ink-3)]">
                        {t('aiDoctor:riskLabel')}
                        {t('aiDoctor:colon')}
                        {rec.riskAssessment}
                      </div>
                    </div>

                    {rec.action && (
                      <button
                        type="button"
                        onClick={() => rec.action && flow.prepare(rec.action)}
                        disabled={flow.busy !== null}
                        className="self-start sm:self-center shrink-0 px-3 py-1.5 rounded text-xs font-medium bg-[var(--ready)] text-white hover:opacity-90 disabled:opacity-50 transition-opacity cursor-pointer focus:outline-none focus:ring-2 focus:ring-[var(--focus)]"
                        aria-label={t('aiDoctor:applyRepair')}
                      >
                        {flow.busy === 'preparing' ? t('aiDoctor:applyingRepair') : t('aiDoctor:applyRepair')}
                      </button>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Privacy & Redaction Seal */}
          <div className="pt-3 border-t border-[var(--border-subtle)] flex flex-col sm:flex-row sm:items-center justify-between gap-2 text-[11px] text-[var(--ink-4)]">
            <span>{t('aiDoctor:redactionSeal')}</span>
            <span>{t('aiDoctor:disclaimer')}</span>
          </div>
        </div>
      )}

      {error && (
        <div className="p-3 rounded bg-[var(--blocked-soft)] text-[var(--blocked-ink)] text-xs font-mono">
          {error}
        </div>
      )}
    </div>
  );
}
