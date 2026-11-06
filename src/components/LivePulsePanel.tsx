import { useState } from 'react';
import { Activity, Cpu, FileCode2, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNexus } from '../app/nexus-context';
import { Section } from './ui';

export function LivePulsePanel() {
  const { t } = useTranslation(['home', 'agents', 'common']);
  const { livePulse, reloadLivePulse } = useNexus();
  const [busy, setBusy] = useState(false);

  if (!livePulse) return null;

  const observedConfigs = livePulse.configLedger.filter(c => c.exists);

  const handleRefresh = async () => {
    setBusy(true);
    try {
      await reloadLivePulse();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Section
      title={t('livePulseTitle')}
      kicker={t('livePulseKicker')}
      className="home-live-pulse"
      aside={
        <button
          type="button"
          className="button button-small"
          disabled={busy}
          onClick={() => void handleRefresh()}
        >
          <RefreshCw size={13} aria-hidden="true" />
          <span>{t('livePulseRefresh')}</span>
        </button>
      }
    >
      <div className="sheet" style={{ padding: '0.85rem', display: 'flex', flexDirection: 'column', gap: '0.85rem', minWidth: 0, maxWidth: '100%', boxSizing: 'border-box', overflow: 'hidden' }}>
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(min(100%, 120px), 1fr))',
            gap: '0.65rem',
            minWidth: 0,
          }}
        >
          <div style={{ padding: '0.75rem', borderRadius: '6px', border: '1px solid var(--border-subtle, rgba(128,128,128,0.2))' }}>
            <div className="kicker">{t('livePulseActiveProcs')}</div>
            <div className="numeral" style={{ fontSize: '1.35rem', fontWeight: 700, marginTop: '0.2rem' }}>
              {livePulse.activeProcessCount}
            </div>
          </div>
          <div style={{ padding: '0.75rem', borderRadius: '6px', border: '1px solid var(--border-subtle, rgba(128,128,128,0.2))' }}>
            <div className="kicker">{t('livePulseTotalMemory')}</div>
            <div className="numeral" style={{ fontSize: '1.35rem', fontWeight: 700, marginTop: '0.2rem' }}>
              {t('livePulseMem', { mem: livePulse.totalMemoryMb.toFixed(1) })}
            </div>
          </div>
          <div style={{ padding: '0.75rem', borderRadius: '6px', border: '1px solid var(--border-subtle, rgba(128,128,128,0.2))' }}>
            <div className="kicker">{t('livePulseTrackedConfigs')}</div>
            <div className="numeral" style={{ fontSize: '1.35rem', fontWeight: 700, marginTop: '0.2rem' }}>
              {observedConfigs.length}
            </div>
          </div>
          <div style={{ padding: '0.75rem', borderRadius: '6px', border: '1px solid var(--border-subtle, rgba(128,128,128,0.2))' }}>
            <div className="kicker">{t('livePulseUniqueSkills')}</div>
            <div className="numeral" style={{ fontSize: '1.35rem', fontWeight: 700, marginTop: '0.2rem' }}>
              {livePulse.skillMatrix.length}
            </div>
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(min(100%, 220px), 1fr))', gap: '0.85rem', minWidth: 0 }}>
          <div style={{ minWidth: 0 }}>
            <h3 className="kicker" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', marginBottom: '0.5rem' }}>
              <Cpu size={13} aria-hidden="true" />
              {t('livePulseProcHeading')}
            </h3>
            {livePulse.processes.length === 0 ? (
              <p className="supporting-note" style={{ margin: 0 }}>{t('livePulseNoProcs')}</p>
            ) : (
              <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: '0.45rem' }}>
                {livePulse.processes.map(proc => (
                  <li
                    key={proc.pid}
                    style={{
                      display: 'flex',
                      flexWrap: 'wrap',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      gap: '0.5rem',
                      padding: '0.5rem 0.65rem',
                      borderRadius: '6px',
                      border: '1px solid var(--border-subtle, rgba(128,128,128,0.18))',
                      fontSize: '0.82rem',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.45rem', flexWrap: 'wrap', minWidth: 0 }}>
                      <Activity size={13} aria-hidden="true" style={{ color: 'var(--status-ready, #16a34a)' }} />
                      <strong lang="en-US">{proc.agentId ? t(`agents:${proc.agentId}`) : t('livePulseMcpHelper')}</strong>
                      <code className="mono" style={{ fontSize: '0.76rem' }}>{proc.command}</code>
                      <span className="mono" style={{ opacity: 0.72, fontSize: '0.75rem' }}>{t('livePulsePid', { pid: proc.pid })}</span>
                    </div>
                    <div className="mono" style={{ display: 'flex', alignItems: 'center', gap: '0.55rem', flexWrap: 'wrap', fontSize: '0.76rem' }}>
                      <span>{t('livePulseMem', { mem: proc.memoryMb.toFixed(1) })}</span>
                      <span>{t('livePulseCpu', { cpu: proc.cpuPercent.toFixed(1) })}</span>
                      <span style={{ opacity: 0.72 }}>{t('livePulseUptime', { elapsed: proc.elapsed })}</span>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div style={{ minWidth: 0 }}>
            <h3 className="kicker" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', marginBottom: '0.5rem' }}>
              <FileCode2 size={13} aria-hidden="true" />
              {t('livePulseConfigHeading')}
            </h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: '0.45rem' }}>
              {observedConfigs.map(cfg => (
                <li
                  key={`${cfg.agentId}:${cfg.location}`}
                  style={{
                    display: 'flex',
                    flexWrap: 'wrap',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    gap: '0.5rem',
                    padding: '0.5rem 0.65rem',
                    borderRadius: '6px',
                    border: '1px solid var(--border-subtle, rgba(128,128,128,0.18))',
                    fontSize: '0.82rem',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.45rem', flexWrap: 'wrap', minWidth: 0 }}>
                    <strong lang="en-US">{t(`agents:${cfg.agentId}`)}</strong>
                    <code className="mono" style={{ fontSize: '0.75rem', overflowWrap: 'anywhere' }}>{cfg.location}</code>
                  </div>
                  <div className="mono" style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexWrap: 'wrap', fontSize: '0.75rem' }}>
                    <span>{t('livePulseLinesBytes', { lines: cfg.lineCount, bytes: cfg.sizeBytes })}</span>
                    {cfg.sha256 ? (
                      <code style={{ opacity: 0.78 }}>{cfg.sha256.slice(0, 12)}</code>
                    ) : null}
                  </div>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </Section>
  );
}
