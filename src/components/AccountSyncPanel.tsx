import { useState } from 'react';
import {
  ShieldCheck,
  Cloud,
  RefreshCw,
  Laptop,
  CheckCircle2,
  Download,
  Trash2,
  Lock,
  Sparkles,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNexus } from '../app/nexus-context';
import type { AccountTier } from '../app/backend';
import { Notice } from './ui';

export function AccountSyncPanel() {
  const { t } = useTranslation(['accounts', 'sync', 'common', 'errors']);
  const {
    account,
    syncDevices,
    syncStatus,
    syncing,
    switchAccountTier,
    triggerSync,
    exportAccountBundle,
    purgeCloudSync,
  } = useNexus();

  const [notice, setNotice] = useState<{ tone: 'success' | 'attention'; message: string } | null>(null);
  const [exporting, setExporting] = useState(false);
  const [purging, setPurging] = useState(false);

  const handleTierSwitch = async (tier: AccountTier) => {
    try {
      await switchAccountTier(tier);
      setNotice({ tone: 'success', message: t('common:saved') });
    } catch {
      setNotice({ tone: 'attention', message: t('errors:operation.failed') });
    }
  };

  const handleSync = async () => {
    try {
      await triggerSync();
      setNotice({ tone: 'success', message: t('sync:statusSynced') });
    } catch {
      setNotice({ tone: 'attention', message: t('errors:operation.failed') });
    }
  };

  const handleExport = async () => {
    setExporting(true);
    try {
      const bundle = await exportAccountBundle();
      const blob = new Blob([bundle], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `nexus-account-export-${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setNotice({ tone: 'success', message: t('accounts:exportSuccess') });
    } catch {
      setNotice({ tone: 'attention', message: t('errors:operation.failed') });
    } finally {
      setExporting(false);
    }
  };

  const handlePurge = async () => {
    if (!window.confirm(t('accounts:purgeConfirm'))) return;
    setPurging(true);
    try {
      await purgeCloudSync();
      setNotice({ tone: 'success', message: t('accounts:purgeSuccess') });
    } catch {
      setNotice({ tone: 'attention', message: t('errors:operation.failed') });
    } finally {
      setPurging(false);
    }
  };

  const currentTier = account?.tier ?? 'free';

  return (
    <div className="account-sync-panel space-y-6">
      {notice && (
        <Notice tone={notice.tone}>
          {notice.message}
        </Notice>
      )}

      {/* Top Banner: Identity & Entitlements */}
      <section className="bg-card border border-border rounded-lg p-5 shadow-sm space-y-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="font-semibold text-base text-foreground">
                {account?.displayName ?? t('accounts:tierFree')}
              </span>
              <span
                className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                  currentTier === 'team'
                    ? 'bg-blue-500/10 text-blue-400 border border-blue-500/30'
                    : currentTier === 'pro'
                    ? 'bg-purple-500/10 text-purple-400 border border-purple-500/30'
                    : 'bg-muted text-muted-foreground border border-border'
                }`}
              >
                {currentTier === 'team'
                  ? t('accounts:tierTeam')
                  : currentTier === 'pro'
                  ? t('accounts:tierPro')
                  : t('accounts:tierFree')}
              </span>
              {account?.isOfflineOnly && (
                <span className="text-xs px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/30">
                  {t('accounts:offlineOnlyBadge')}
                </span>
              )}
            </div>
            <p className="text-xs text-muted-foreground">{account?.email}</p>
          </div>

          {/* Tier Switcher for development/testing */}
          <div className="flex flex-wrap items-center gap-1 bg-muted p-1 rounded-md text-xs">
            <button
              type="button"
              onClick={() => handleTierSwitch('free')}
              className={`px-2.5 py-1 rounded transition-colors ${
                currentTier === 'free'
                  ? 'bg-card text-foreground shadow-xs font-medium'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {t('accounts:tierFree')}
            </button>
            <button
              type="button"
              onClick={() => handleTierSwitch('pro')}
              className={`px-2.5 py-1 rounded transition-colors ${
                currentTier === 'pro'
                  ? 'bg-card text-foreground shadow-xs font-medium'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {t('accounts:tierPro')}
            </button>
            <button
              type="button"
              onClick={() => handleTierSwitch('team')}
              className={`px-2.5 py-1 rounded transition-colors ${
                currentTier === 'team'
                  ? 'bg-card text-foreground shadow-xs font-medium'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {t('accounts:tierTeam')}
            </button>
          </div>
        </div>

        {/* Offline Safety Reassurance */}
        <div className="flex items-start gap-3 bg-muted/40 border border-border/60 rounded-md p-3 text-xs">
          <ShieldCheck className="w-4 h-4 text-emerald-500 shrink-0 mt-0.5" />
          <div className="space-y-0.5">
            <span className="font-medium text-foreground">
              {t('accounts:offlineGuaranteeTitle')}
            </span>
            <p className="text-muted-foreground leading-relaxed">
              {t('accounts:offlineGuaranteeDesc')}
            </p>
          </div>
        </div>

        {/* Entitlements Grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3 pt-2 border-t border-border/50 text-xs">
          <div className="space-y-0.5">
            <span className="text-muted-foreground">{t('accounts:retentionLabel')}</span>
            <div className="font-medium text-foreground">
              {t('accounts:retentionDays', { days: account?.backupRetentionDays ?? 7 })}
            </div>
          </div>
          <div className="space-y-0.5">
            <span className="text-muted-foreground">{t('accounts:maxOperationsLabel')}</span>
            <div className="font-medium text-foreground">
              {t('accounts:maxOperationsValue', {
                count: currentTier === 'team' ? 10000 : currentTier === 'pro' ? 1000 : 10,
              })}
            </div>
          </div>
          <div className="space-y-0.5">
            <span className="text-muted-foreground">{t('accounts:cloudSyncStatusLabel')}</span>
            <div className="font-medium text-foreground flex items-center gap-1.5">
              {account?.cloudSyncEnabled ? (
                <>
                  <CheckCircle2 className="w-3.5 h-3.5 text-purple-400" />
                  <span>{t('accounts:cloudSyncEnabled')}</span>
                </>
              ) : (
                <>
                  <Lock className="w-3.5 h-3.5 text-muted-foreground" />
                  <span>{t('accounts:cloudSyncDisabled')}</span>
                </>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* Cloud Mesh Sync Section */}
      <section className="bg-card border border-border rounded-lg p-5 shadow-sm space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <Cloud className="w-4 h-4 text-purple-400" />
              <h2 className="font-semibold text-base text-foreground">
                {t('sync:title')}
              </h2>
            </div>
            <p className="text-xs text-muted-foreground">
              {t('sync:subtitle')}
            </p>
          </div>

          <button
            type="button"
            onClick={handleSync}
            disabled={syncing}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${syncing ? 'animate-spin' : ''}`} />
            <span>{syncing ? t('sync:syncing') : t('sync:syncNow')}</span>
          </button>
        </div>

        {/* Sync Summary Provenance */}
        <div className="bg-muted/30 border border-border/50 rounded-md p-3 text-xs space-y-2">
          <div className="flex flex-wrap items-center justify-between gap-2 text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <Lock className="w-3.5 h-3.5 text-purple-400" />
              <span className="font-medium text-foreground">
                {t('sync:provenanceNotice')}
              </span>
            </div>
            <span>
              {syncStatus?.timestamp
                ? t('sync:lastSynced', { time: syncStatus.timestamp.slice(0, 19).replace('T', ' ') })
                : t('sync:neverSynced')}
            </span>
          </div>
          <p className="text-muted-foreground/80 leading-relaxed">
            {t('sync:zeroSecretsGuarantee')}
          </p>
        </div>

        {/* Paired Workstations Mesh */}
        <div className="space-y-2">
          <div className="text-xs font-medium text-foreground flex items-center justify-between">
            <span>
              {t('sync:pairedDevicesTitle', { count: syncDevices.length })}
            </span>
            {syncStatus && (
              <span className="text-muted-foreground">
                {t('sync:envelopesCount', { count: syncStatus.envelopesSynced })}
              </span>
            )}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {syncDevices.map(device => (
              <div
                key={device.deviceId}
                className={`border rounded-md p-3.5 flex items-start justify-between text-xs transition-colors ${
                  device.isCurrent
                    ? 'border-purple-500/40 bg-purple-500/5'
                    : 'border-border bg-card'
                }`}
              >
                <div className="space-y-1.5">
                  <div className="flex items-center gap-2">
                    <Laptop className="w-4 h-4 text-muted-foreground" />
                    <span className="font-medium text-foreground">
                      {device.deviceName}
                    </span>
                    {device.isCurrent && (
                      <span className="text-[10px] px-1.5 py-0.2 bg-purple-500/20 text-purple-300 rounded font-medium">
                        {t('sync:currentDeviceBadge')}
                      </span>
                    )}
                  </div>
                  <div className="text-muted-foreground flex items-center gap-2 text-[11px]">
                    <span className="capitalize">{device.platform}</span>
                    <span className="note-divider" aria-hidden="true" />
                    <span>{t('sync:lastSynced', { time: device.lastSyncedAt.slice(0, 10) })}</span>
                  </div>
                </div>

                <span className="flex items-center gap-1 text-[11px] text-emerald-400 font-medium">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                  <span>{t('sync:statusSynced')}</span>
                </span>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Pro Data Rights & Privacy */}
      <section className="bg-card border border-border rounded-lg p-5 shadow-sm space-y-4">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-purple-400" />
            <h2 className="font-semibold text-base text-foreground">
              {t('accounts:exportTitle')}
            </h2>
          </div>
          <p className="text-xs text-muted-foreground">
            {t('accounts:exportDesc')}
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-3 pt-2">
          <button
            type="button"
            onClick={handleExport}
            disabled={exporting}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium border border-border bg-muted/40 hover:bg-muted text-foreground transition-colors disabled:opacity-50"
          >
            <Download className="w-3.5 h-3.5" />
            <span>{t('accounts:exportButton')}</span>
          </button>

          <button
            type="button"
            onClick={handlePurge}
            disabled={purging}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium border border-destructive/40 text-destructive hover:bg-destructive/10 transition-colors disabled:opacity-50"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>{t('accounts:purgeButton')}</span>
          </button>
        </div>
      </section>
    </div>
  );
}
