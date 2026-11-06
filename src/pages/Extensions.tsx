import { useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNexus } from '../app/nexus-context';
import { usePreferences } from '../app/preference-context';
import { formatNumber } from '../app/format';
import { ExtensionRows } from '../components/ExtensionRows';
import { CrossAgentSkillMatrix } from '../components/CrossAgentSkillMatrix';
import { MarketplacePanel } from '../components/MarketplacePanel';
import { ActionPanel } from '../components/ActionFlow';
import { useActionFlow } from '../app/action-flow';
import type { AgentId } from '../app/backend';
import { DataState, Notice, PageHeading, Tabs } from '../components/ui';

export function Extensions() {
  const { t } = useTranslation(['extensions', 'navigation', 'common', 'agents', 'registry']);
  const { locale } = usePreferences();
  const { extensions, registryExtensions, mode, status } = useNexus();
  const flow = useActionFlow();
  const [view, setView] = useState<'mcp' | 'skills' | 'marketplace'>('mcp');
  const [skillAgent, setSkillAgent] = useState<AgentId>('gemini');
  const [skillDir, setSkillDir] = useState('');
  const live = mode === 'live';
  const byView = (value: 'mcp' | 'skills') => extensions.filter(item => item.type === (value === 'mcp' ? 'mcp' : 'skill'));

  const tabOptions = [
    { value: 'mcp' as const, label: t('mcp'), count: formatNumber(byView('mcp').length, locale) },
    { value: 'skills' as const, label: t('skills'), count: formatNumber(byView('skills').length, locale) },
    { value: 'marketplace' as const, label: t('marketplace'), count: formatNumber(registryExtensions.length, locale) },
  ];

  return <><PageHeading title={t('navigation:extensions')} description={t('description')} kicker={t(live ? 'pageKickerLive' : 'pageKicker')} />
    {status !== 'ready' ? <DataState /> : <Tabs label={t('tabs')} value={view} onChange={setView} options={tabOptions}>
      <p className="view-intro">{t(`${view}Help${live ? 'Live' : ''}`)}</p>

      {view === 'marketplace' ? (
        <MarketplacePanel />
      ) : (
        <>
          <ExtensionRows items={byView(view)} actions={live ? item => {
            const server = item.live?.server;
            const skill = item.live?.skill;
            if (!item.live) return null;
            const agentId = item.live.agentId;
            if (server) {
              return <div className="row-actions">
                {server.canToggle && server.enabled !== null && <button className="button button-small" disabled={flow.busy !== null} onClick={() => void flow.prepare({ kind: 'setMcpEnabled', agentId, name: server.name, enabled: !server.enabled })}>{t(server.enabled ? 'liveDisable' : 'liveEnable')}</button>}
                {server.nexusManaged && <button className="button button-small" disabled={flow.busy !== null} onClick={() => void flow.prepare({ kind: 'removeManagedMcp', agentId, name: server.name })}>{t('liveRemoveManaged')}</button>}
              </div>;
            }
            if (skill?.nexusManaged) {
              return <div className="row-actions">
                <button className="button button-small" disabled={flow.busy !== null} onClick={() => void flow.prepare({ kind: 'removeManagedSkill', agentId, name: skill.name })}>{t('liveRemoveSkill')}</button>
              </div>;
            }
            return null;
          } : undefined} />

          {view === 'skills' && <CrossAgentSkillMatrix />}

          {live && view === 'skills' && <section className="sheet add-mcp" aria-labelledby="add-skill-title">
            <header className="sheet-head"><h2 id="add-skill-title" className="sheet-title">{t('liveAddSkillTitle')}</h2><span className="kicker">{t('liveAddSkillKicker')}</span></header>
            <div className="sheet-body form-stack">
              <div className="setting-row"><div><label className="setting-label" htmlFor="skill-target-agent">{t('liveAddSkillAgent')}</label><p className="field-help">{t('liveAddSkillAgentHelp')}</p></div>
                <span className="select-control"><select id="skill-target-agent" value={skillAgent} onChange={event => setSkillAgent(event.target.value as AgentId)}>{(['codex', 'claude', 'gemini', 'opencode'] as const).map(agent => <option key={agent} value={agent}>{t(`agents:${agent}`)}</option>)}</select><ChevronDown size={14} aria-hidden="true" /></span></div>
              <div className="setting-row"><div><label className="setting-label" htmlFor="skill-source-dir">{t('liveAddSkillDir')}</label><p className="field-help">{t('liveAddSkillDirHelp')}</p></div><span className="search-field"><input id="skill-source-dir" className="mono" value={skillDir} maxLength={1024} placeholder={t('skillDirPlaceholder')} onChange={event => setSkillDir(event.target.value)} /></span></div>
              <div className="plan-actions"><button className="button" disabled={!skillDir.trim() || flow.busy !== null} onClick={() => void flow.prepare({ kind: 'addSkill', agentId: skillAgent, sourceDirectory: skillDir.trim() })}>{t('liveAddSkillPreview')}</button></div>
              <p className="field-help">{t('liveAddSkillNotice')}</p>
            </div>
          </section>}
        </>
      )}

      {live ? <><ActionPanel flow={flow} /><Notice>{t(view === 'mcp' ? 'liveMcpBoundary' : view === 'skills' ? 'liveSkillsBoundary' : 'registry:offlineNotice')}</Notice></> : <Notice>{t('common:previewOnly')}</Notice>}
    </Tabs>}
  </>;
}
