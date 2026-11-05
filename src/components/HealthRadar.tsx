import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { HealthRadarStage } from '../app/backend';

interface HealthRadarProps {
  stages: HealthRadarStage[];
  compact?: boolean;
}

export function HealthRadar({ stages, compact = false }: HealthRadarProps) {
  const { t } = useTranslation();
  const [selectedStageId, setSelectedStageId] = useState<string | null>(null);

  const selectedStage = stages.find(s => s.id === selectedStageId);

  const getStageLabel = (id: string) => {
    switch (id) {
      case 'runtime': return t('aiDoctor:stageRuntime');
      case 'toolchain': return t('aiDoctor:stageToolchain');
      case 'config': return t('aiDoctor:stageConfig');
      case 'auth': return t('aiDoctor:stageAuth');
      case 'model': return t('aiDoctor:stageModel');
      case 'extensions': return t('aiDoctor:stageExtensions');
      case 'network': return t('aiDoctor:stageNetwork');
      default: return id;
    }
  };

  const getOutcomeTheme = (outcome: HealthRadarStage['outcome']) => {
    switch (outcome) {
      case 'pass':
        return {
          bg: 'var(--ready-soft)',
          border: 'var(--ready)',
          text: 'var(--ready-ink)',
          badge: t('aiDoctor:stageStatusPass'),
          glyph: '✓',
        };
      case 'fail':
        return {
          bg: 'var(--blocked-soft)',
          border: 'var(--blocked)',
          text: 'var(--blocked-ink)',
          badge: t('aiDoctor:stageStatusFail'),
          glyph: '✕',
        };
      case 'unknown':
        return {
          bg: 'var(--attention-soft)',
          border: 'var(--attention)',
          text: 'var(--attention-ink)',
          badge: t('aiDoctor:stageStatusUnknown'),
          glyph: '!',
        };
      case 'notApplicable':
      default:
        return {
          bg: 'var(--unknown-soft)',
          border: 'var(--border)',
          text: 'var(--ink-3)',
          badge: t('aiDoctor:stageStatusNotApplicable'),
          glyph: '—',
        };
    }
  };

  if (compact) {
    return (
      <div className="flex items-center gap-1.5 py-1">
        {stages.map((stage) => {
          const theme = getOutcomeTheme(stage.outcome);
          const label = getStageLabel(stage.id);
          return (
            <span
              key={stage.id}
              className="w-2.5 h-2.5 rounded-full border transition-transform duration-200 hover:scale-125"
              style={{ backgroundColor: theme.bg, borderColor: theme.border }}
              aria-label={label}
            />
          );
        })}
      </div>
    );
  }

  return (
    <div className="p-4 rounded-lg border border-[var(--border)] bg-[var(--sheet)] shadow-[var(--shadow-sheet)] mb-4">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 mb-4 pb-3 border-b border-[var(--border-subtle)]">
        <div>
          <h3 className="text-sm font-semibold tracking-wide text-[var(--ink)]">
            {t('aiDoctor:radarTitle')}
          </h3>
          <p className="text-xs text-[var(--ink-3)]">
            {t('aiDoctor:radarSubtitle')}
          </p>
        </div>
      </div>

      {/* 7-Stage Signal Bus Line */}
      <div className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-7 gap-2">
        {stages.map((stage) => {
          const theme = getOutcomeTheme(stage.outcome);
          const isSelected = selectedStageId === stage.id;
          const label = getStageLabel(stage.id);

          return (
            <button
              key={stage.id}
              type="button"
              onClick={() => setSelectedStageId(isSelected ? null : stage.id)}
              className="flex flex-col items-center justify-between p-2.5 rounded-md border text-center transition-all duration-150 cursor-pointer focus:outline-none focus:ring-2 focus:ring-[var(--focus)]"
              style={{
                backgroundColor: isSelected ? 'var(--selected)' : 'var(--raised)',
                borderColor: isSelected ? 'var(--brand)' : theme.border,
              }}
              aria-label={label}
            >
              <div className="flex items-center justify-center w-6 h-6 rounded-full font-mono text-xs font-bold mb-1.5"
                   style={{ backgroundColor: theme.bg, color: theme.text, borderColor: theme.border }}>
                {theme.glyph}
              </div>
              <span className="text-xs font-medium text-[var(--ink)] line-clamp-1">
                {label}
              </span>
              <span className="text-[10px] mt-1 px-1.5 py-0.5 rounded font-mono"
                    style={{ backgroundColor: theme.bg, color: theme.text }}>
                {theme.badge}
              </span>
            </button>
          );
        })}
      </div>

      {/* Stage Inspection Details Panel */}
      {selectedStage && (
        <div className="mt-4 p-3 rounded-md bg-[var(--sunken)] border border-[var(--border)] animate-in fade-in duration-200">
          <div className="flex items-center justify-between mb-2">
            <h4 className="text-xs font-semibold uppercase tracking-wider text-[var(--ink-2)]">
              {getStageLabel(selectedStage.id)}
              {t('aiDoctor:colon')}
              {selectedStage.summary}
            </h4>
            <button
              type="button"
              onClick={() => setSelectedStageId(null)}
              className="text-xs text-[var(--ink-4)] hover:text-[var(--ink)] p-1 rounded"
              aria-label={t('aiDoctor:closeStageDetails')}
            >
              {t('aiDoctor:closeStageDetails')}
            </button>
          </div>
          <p className="text-xs text-[var(--ink-3)] mb-2 font-mono">
            {selectedStage.checkIds.join(', ')}
          </p>
        </div>
      )}
    </div>
  );
}
