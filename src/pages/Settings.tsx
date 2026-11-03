import { useEffect, useId, useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { backend, NexusError, type DiagnosticPreview, type StorageStatus } from '../app/backend';
import { formatBytes, formatNumber } from '../app/format';
import { usePreferences } from '../app/preference-context';
import { useNexus } from '../app/nexus-context';
import { InfoList, Notice, PageHeading, StateGlyph } from '../components/ui';

const themes = ['light', 'dark', 'system'] as const;
const groups = ['appearance', 'privacy', 'diagnostics', 'about'] as const;

// A specimen is real Nexus UI scoped to one palette with data-theme, not a picture of one.
function Plate({ theme }: { theme: 'light' | 'dark' }) {
  const { t } = useTranslation(['health', 'home']);
  return <span className="specimen-plate" data-theme={theme}>
    <span className="plate-kicker">{t('home:specimenKicker')}</span>
    <span className="plate-line"><span className="lead">{t('home:specimenLead')}</span></span>
    <span className="plate-nodes" aria-hidden="true"><i data-n="pass" /><i data-n="pass" /><i data-n="pass" /><i data-n="pending" /><i data-n="unchecked" /></span>
    <span className="seal" data-s="attention"><StateGlyph state="attention" size={9} />{t('attention')}</span>
  </span>;
}

export function Settings() {
  const { t } = useTranslation(['settings', 'navigation', 'common', 'errors']);
  const { preferences, setTheme, storageFailed, version, locale } = usePreferences();
  const { mode } = useNexus();
  const live = mode === 'live';
  const id = useId();
  const [storage, setStorage] = useState<StorageStatus | null>(null);
  const [storageUnavailable, setStorageUnavailable] = useState(false);
  const [retainDays, setRetainDays] = useState<'30' | '7' | '0'>('30');
  const [pruning, setPruning] = useState(false);
  const [pruneMessage, setPruneMessage] = useState<{ tone: 'success' | 'error'; text: string } | null>(null);
  const [diagBusy, setDiagBusy] = useState(false);
  const [diagPreview, setDiagPreview] = useState<DiagnosticPreview | null>(null);
  const [diagMessage, setDiagMessage] = useState<{ tone: 'success' | 'error'; text: string } | null>(null);

  useEffect(() => {
    if (!live) return;
    let cancelled = false;
    backend.storageStatus()
      .then(data => { if (!cancelled) { setStorage(data); setStorageUnavailable(false); } })
      .catch(() => { if (!cancelled) setStorageUnavailable(true); });
    return () => { cancelled = true; };
  }, [live]);

  async function handlePrune() {
    setPruning(true);
    setPruneMessage(null);
    try {
      const res = await backend.pruneHistory(Number(retainDays));
      const updated = await backend.storageStatus();
      setStorage(updated);
      setPruneMessage({ tone: 'success', text: t('pruneDone', { ops: formatNumber(res.prunedOperations, locale), health: formatNumber(res.prunedHealthRows, locale), backups: formatNumber(res.prunedBackups, locale) }) });
    } catch (caught) {
      setPruneMessage({ tone: 'error', text: t(`errors:${caught instanceof NexusError ? caught.code : 'ipc.unavailable'}`, caught instanceof NexusError ? caught.params : undefined) });
    } finally {
      setPruning(false);
    }
  }

  async function handlePreviewDiagnostics() {
    setDiagBusy(true);
    setDiagMessage(null);
    try {
      const preview = await backend.diagnosticPreview();
      setDiagPreview(preview);
    } catch (caught) {
      setDiagMessage({ tone: 'error', text: t(`errors:${caught instanceof NexusError ? caught.code : 'ipc.unavailable'}`, caught instanceof NexusError ? caught.params : undefined) });
    } finally {
      setDiagBusy(false);
    }
  }

  async function handleCopyDiagnostics() {
    if (!diagPreview) return;
    try {
      await navigator.clipboard.writeText(diagPreview.json);
      setDiagMessage({ tone: 'success', text: t('copiedDiagnostics') });
    } catch {
      setDiagMessage({ tone: 'error', text: t('copyFailed') });
    }
  }

  function handleDownloadDiagnostics() {
    if (!diagPreview) return;
    const blob = new Blob([diagPreview.json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = diagPreview.filename;
    link.click();
    URL.revokeObjectURL(url);
    setDiagMessage({ tone: 'success', text: t('downloadedDiagnostics', { filename: diagPreview.filename }) });
  }

  return <><PageHeading title={t('navigation:settings')} description={t('description')} kicker={t('pageKicker')} />
    <div className="settings-layout">
      <nav className="settings-categories" aria-label={t('contents')}><p className="kicker">{t('contents')}</p>{groups.map((group, index) => <button key={group} onClick={() => document.getElementById(`${id}-${group}`)?.scrollIntoView({ block: 'start' })}><span className="mono" aria-hidden="true">{t('common:sectionMark', { n: index + 1 })}</span>{t(group === 'about' && live ? 'aboutLive' : group)}</button>)}<p className="categories-note">{t('saved')}</p></nav>
      <div className="settings-content">
        <section className="settings-group" id={`${id}-appearance`} aria-labelledby={`${id}-appearance-title`}>
          <div className="group-head"><h2 id={`${id}-appearance-title`}><span className="section-index" aria-hidden="true">{t('common:sectionMark', { n: 1 })}</span>{t('appearance')}</h2><span className="kicker">{t('appliesNow')}</span></div>
          <fieldset><legend className="sr-only">{t('theme')}</legend><p id="theme-help" className="field-help">{t('appearanceHelp')}</p>
            <div className="theme-options" aria-describedby="theme-help">{themes.map(theme => <label className={`theme-option ${preferences.theme === theme ? 'is-selected' : ''}`} key={theme}>
              <input type="radio" name="theme" value={theme} checked={preferences.theme === theme} onChange={() => setTheme(theme)} aria-labelledby={`${id}-${theme}-name`} aria-describedby={`${id}-${theme}-note`} />
              <span className={`specimen specimen-${theme}`} aria-hidden="true">{theme === 'system' ? <><Plate theme="light" /><Plate theme="dark" /></> : <Plate theme={theme} />}</span>
              <span className="theme-caption"><span className="theme-name" id={`${id}-${theme}-name`}>{t(theme)}</span><span className="theme-note" id={`${id}-${theme}-note`}>{t(`${theme}Caption`)}</span><span className="theme-radio" aria-hidden="true" /></span>
            </label>)}</div>
          </fieldset>
          <p className="preference-feedback" role="status">{storageFailed ? t('common:storageError') : t('saved')}</p>
        </section>
        <section className="settings-group" id={`${id}-privacy`} aria-labelledby={`${id}-privacy-title`}>
          <div className="group-head"><h2 id={`${id}-privacy-title`}><span className="section-index" aria-hidden="true">{t('common:sectionMark', { n: 2 })}</span>{t('privacy')}</h2><span className="kicker">{t('privacyKicker')}</span></div>
          <p className="field-help">{t('privacyHelp')}</p>
          {!live ? <Notice>{t('privacyDemoHelp')}</Notice> : storageUnavailable ? <Notice>{t('privacyUnavailable')}</Notice> : storage && <>
            <InfoList items={[
              { label: t('storageLocation'), value: <span className="mono">{storage.dataDir}</span> },
              { label: t('storageDb'), value: <span className="mono">{formatBytes(storage.dbBytes, locale)}</span> },
              { label: t('storageBackups'), value: t('storageBackupsValue', { count: storage.backupCount, size: formatBytes(storage.backupBytes, locale) }) },
              { label: t('storageOperations'), value: t('storageOperationsValue', { count: storage.operationCount }) },
              { label: t('storageHealthRows'), value: t('storageHealthValue', { count: storage.healthRowCount }) },
            ]} />
            <div className="sheet settings-sheet"><div className="sheet-body form-stack">
              <div className="setting-row"><div><label className="setting-label" htmlFor={`${id}-retain`}>{t('retainWindow')}</label><p className="field-help">{t('retainWindowHelp')}</p></div>
                <span className="select-control"><select id={`${id}-retain`} value={retainDays} onChange={event => setRetainDays(event.target.value as '30' | '7' | '0')}><option value="30">{t('retain30')}</option><option value="7">{t('retain7')}</option><option value="0">{t('retain0')}</option></select><ChevronDown size={14} aria-hidden="true" /></span></div>
              <div className="plan-actions"><button className="button" disabled={pruning} onClick={() => void handlePrune()}>{t('pruneButton')}</button></div>
              {pruneMessage && <Notice tone={pruneMessage.tone}>{pruneMessage.text}</Notice>}
            </div></div>
          </>}
        </section>
        <section className="settings-group" id={`${id}-diagnostics`} aria-labelledby={`${id}-diagnostics-title`}>
          <div className="group-head"><h2 id={`${id}-diagnostics-title`}><span className="section-index" aria-hidden="true">{t('common:sectionMark', { n: 3 })}</span>{t('diagnostics')}</h2><span className="kicker">{t('diagnosticsKicker')}</span></div>
          <p className="field-help">{t('diagnosticsHelp')}</p>
          {!live ? <Notice>{t('diagnosticsDemoHelp')}</Notice> : <>
            <div className="plan-actions">
              <button className="button" disabled={diagBusy} onClick={() => void handlePreviewDiagnostics()}>{t(diagPreview ? 'refreshPreview' : 'previewDiagnostics')}</button>
              {diagPreview && <><button className="button" onClick={() => void handleCopyDiagnostics()}>{t('copyDiagnostics')}</button><button className="button" onClick={handleDownloadDiagnostics}>{t('downloadDiagnostics')}</button></>}
            </div>
            {diagMessage && <Notice tone={diagMessage.tone}>{diagMessage.text}</Notice>}
            {diagPreview && <pre className="plan-diff mono" tabIndex={0} aria-label={t('diagnostics')}>{diagPreview.json}</pre>}
          </>}
        </section>
        <section className="settings-group" id={`${id}-about`} aria-labelledby={`${id}-about-title`}>
          <div className="group-head"><h2 id={`${id}-about-title`}><span className="section-index" aria-hidden="true">{t('common:sectionMark', { n: 4 })}</span>{t(live ? 'aboutLive' : 'about')}</h2><span className="kicker">{t(live ? 'common:phaseLive' : 'common:phase')}</span></div>
          <p className="field-help">{t(live ? 'aboutHelpLive' : 'aboutHelp')}</p>
          <InfoList items={[{ label: t('version'), value: <span className="mono">{version}</span> }, { label: t('build'), value: t(live ? 'common:phaseLive' : 'common:phase') }, ...(live ? [{ label: t('dataMode'), value: t('dataModeLive') }] : [])]} />
        </section>
      </div>
    </div>
  </>;
}
