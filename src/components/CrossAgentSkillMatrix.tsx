import { ArrowRightLeft, CheckCircle2, Sparkles } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useActionFlow } from '../app/action-flow';
import type { AgentId } from '../app/backend';
import { useNexus } from '../app/nexus-context';
import { Section } from './ui';

export function CrossAgentSkillMatrix() {
  const { t } = useTranslation(['extensions', 'agents', 'common']);
  const { livePulse, reloadLivePulse } = useNexus();
  const flow = useActionFlow();

  if (!livePulse || livePulse.skillMatrix.length === 0) return null;

  const handlePortSkill = async (targetAgent: AgentId, sourceDirectory: string) => {
    await flow.prepare({
      kind: 'addSkill',
      agentId: targetAgent,
      sourceDirectory,
    });
    await reloadLivePulse();
  };

  return (
    <Section
      title={t('skillMatrixTitle')}
      kicker={t('skillMatrixKicker')}
      className="cross-agent-skill-matrix"
    >
      <p className="section-help" style={{ marginTop: 0 }}>{t('skillMatrixSubtitle')}</p>
      <div className="sheet" style={{ display: 'flex', flexDirection: 'column', minWidth: 0, maxWidth: '100%', boxSizing: 'border-box', overflow: 'hidden' }}>
        {livePulse.skillMatrix.map((skill, idx) => (
          <div
            key={skill.name}
            style={{
              padding: '0.85rem 1rem',
              borderTop: idx === 0 ? 'none' : '1px solid var(--border-subtle, rgba(128,128,128,0.18))',
              display: 'flex',
              flexWrap: 'wrap',
              alignItems: 'flex-start',
              justifyContent: 'space-between',
              gap: '0.75rem',
            }}
          >
            <div style={{ flex: '1 1 240px', minWidth: 0 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexWrap: 'wrap' }}>
                <Sparkles size={14} aria-hidden="true" />
                <strong className="mono" style={{ fontSize: '0.92rem' }} lang="en-US">{skill.name}</strong>
                <span
                  className="mono"
                  style={{
                    fontSize: '0.72rem',
                    padding: '0.1rem 0.4rem',
                    borderRadius: '4px',
                    border: '1px solid var(--border-subtle, rgba(128,128,128,0.3))',
                  }}
                >
                  {skill.hasScripts ? t('skillScriptsBadge') : t('skillInstructionsBadge')}
                </span>
              </div>
              {skill.description ? (
                <p style={{ margin: '0.3rem 0', fontSize: '0.83rem', opacity: 0.88 }}>
                  {skill.description}
                </p>
              ) : null}
              <code className="mono" style={{ fontSize: '0.74rem', opacity: 0.72, overflowWrap: 'anywhere', display: 'block' }}>
                {skill.sourceLocation}
              </code>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.45rem', alignItems: 'flex-start', flex: '1 1 220px', minWidth: 0 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.35rem', flexWrap: 'wrap', fontSize: '0.78rem' }}>
                <span className="field-label">{t('skillInstalledOn')}</span>
                {skill.installedOn.map(aid => (
                  <span
                    key={aid}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: '0.25rem',
                      padding: '0.12rem 0.45rem',
                      borderRadius: '4px',
                      background: 'rgba(22, 163, 74, 0.12)',
                      fontSize: '0.76rem',
                      fontWeight: 600,
                    }}
                    lang="en-US"
                  >
                    <CheckCircle2 size={11} aria-hidden="true" />
                    {t(`agents:${aid}`)}
                  </span>
                ))}
              </div>

              {skill.missingOn.length > 0 ? (
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.35rem', flexWrap: 'wrap', fontSize: '0.78rem' }}>
                  <span className="field-label">{t('skillTeleportTo')}</span>
                  {skill.missingOn.map(targetAgent => (
                    <button
                      key={targetAgent}
                      type="button"
                      className="button button-small"
                      disabled={flow.busy !== null}
                      onClick={() => void handlePortSkill(targetAgent, skill.sourceLocation)}
                    >
                      <ArrowRightLeft size={11} aria-hidden="true" />
                      <span>{t('skillPortButton', { agent: t(`agents:${targetAgent}`) })}</span>
                    </button>
                  ))}
                </div>
              ) : (
                <span style={{ fontSize: '0.76rem', opacity: 0.75 }}>{t('skillAllInstalled')}</span>
              )}
            </div>
          </div>
        ))}
      </div>
    </Section>
  );
}
