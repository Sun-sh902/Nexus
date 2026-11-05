import { useState, useMemo, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Search,
  ShieldCheck,
  ShieldAlert,
  CheckCircle2,
  AlertTriangle,
  Key,
  RefreshCw,
  FileCode,
  X,
  Lock,
} from 'lucide-react';
import { useNexus } from '../app/nexus-context';
import { useActionFlow } from '../app/action-flow';
import type {
  AgentId,
  ExtensionPermissionAnalysis,
  RegistryExtension,
  SignatureVerificationResult,
} from '../app/backend';

export function MarketplacePanel() {
  const { t } = useTranslation(['registry', 'common', 'agents']);
  const {
    registryExtensions,
    registryLoading,
    reloadRegistry,
    refreshRegistry,
    analyzePermissions,
    verifySignature,
  } = useNexus();
  const flow = useActionFlow();

  const [searchQuery, setSearchQuery] = useState('');
  const [selectedTier, setSelectedTier] = useState<string>('all');
  const [inspectingExtension, setInspectingExtension] = useState<RegistryExtension | null>(null);
  const [targetAgent, setTargetAgent] = useState<AgentId>('gemini');
  const [permissionAnalysis, setPermissionAnalysis] = useState<ExtensionPermissionAnalysis | null>(null);
  const [signatureResult, setSignatureResult] = useState<SignatureVerificationResult | null>(null);
  const [escalationConfirmed, setEscalationConfirmed] = useState(false);
  const isAnalyzing = inspectingExtension !== null && (permissionAnalysis === null || signatureResult === null);

  // Trigger search / filter reload
  useEffect(() => {
    void reloadRegistry(searchQuery, selectedTier);
  }, [searchQuery, selectedTier, reloadRegistry]);

  const handleOpenInspect = useCallback((ext: RegistryExtension) => {
    setInspectingExtension(ext);
    setPermissionAnalysis(null);
    setSignatureResult(null);
    setEscalationConfirmed(false);
  }, []);

  const handleCloseInspect = useCallback(() => {
    setInspectingExtension(null);
    setPermissionAnalysis(null);
    setSignatureResult(null);
    setEscalationConfirmed(false);
  }, []);

  // When inspecting extension changes or target agent changes, analyze permissions and signature
  useEffect(() => {
    if (!inspectingExtension) return;
    let cancelled = false;
    void Promise.all([
      analyzePermissions(inspectingExtension.id, targetAgent),
      verifySignature(inspectingExtension.id),
    ]).then(([analysis, sig]) => {
      if (!cancelled) {
        setPermissionAnalysis(analysis);
        setSignatureResult(sig);
        setEscalationConfirmed(!analysis.hasPrivilegeEscalation);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [inspectingExtension, targetAgent, analyzePermissions, verifySignature]);

  // Handle escape key to close modal
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && inspectingExtension) {
        handleCloseInspect();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [inspectingExtension, handleCloseInspect]);

  const handleInstall = useCallback(() => {
    if (!inspectingExtension || !permissionAnalysis || permissionAnalysis.isRevoked) return;
    if (inspectingExtension.extensionType === 'mcp') {
      const recipeId = inspectingExtension.id.replace('mcp-', '');
      void flow.prepare({
        kind: 'addReviewedMcp',
        agentId: targetAgent,
        recipeId: recipeId === 'memory' || recipeId === 'filesystem' ? recipeId : 'memory',
        name: inspectingExtension.name,
        directory: undefined,
      });
    } else {
      void flow.prepare({
        kind: 'addSkill',
        agentId: targetAgent,
        sourceDirectory: `~/.nexus/skills/${inspectingExtension.name}`,
      });
    }
    handleCloseInspect();
  }, [inspectingExtension, permissionAnalysis, targetAgent, flow, handleCloseInspect]);

  const filteredExtensions = useMemo(() => {
    return registryExtensions.filter((ext) => {
      if (selectedTier === 'revoked') {
        return ext.reviewLevel === 'revoked' || ext.signatureStatus === 'revoked';
      }
      if (selectedTier !== 'all' && ext.publisher.trustTier !== selectedTier) {
        return false;
      }
      if (searchQuery.trim()) {
        const q = searchQuery.trim().toLowerCase();
        return (
          ext.name.toLowerCase().includes(q) ||
          ext.displayName.toLowerCase().includes(q) ||
          ext.description.toLowerCase().includes(q) ||
          ext.publisher.displayName.toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [registryExtensions, selectedTier, searchQuery]);

  return (
    <section className="marketplace-container" aria-labelledby="marketplace-heading">
      <header className="marketplace-header">
        <div>
          <h2 id="marketplace-heading" className="sheet-title">
            {t('registry:title')}
          </h2>
          <p className="field-help">{t('registry:description')}</p>
        </div>
        <div className="marketplace-sync-bar">
          <span className="badge badge-subtle">
            <Lock size={12} aria-hidden="true" />
            {t('registry:offlineNotice')}
          </span>
          <button
            type="button"
            className="button button-small"
            disabled={registryLoading}
            onClick={() => void refreshRegistry(true)}
            aria-label={t('registry:refreshIndex')}
          >
            <RefreshCw
              size={13}
              className={registryLoading ? 'spin' : ''}
              aria-hidden="true"
            />
            <span>{t(registryLoading ? 'registry:refreshingIndex' : 'registry:refreshIndex')}</span>
          </button>
        </div>
      </header>

      {/* Search and Filter Chips */}
      <div className="marketplace-controls">
        <div className="search-field" style={{ flex: '1 1 260px' }}>
          <Search size={14} className="search-icon" aria-hidden="true" />
          <input
            type="search"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t('registry:searchPlaceholder')}
            aria-label={t('registry:searchPlaceholder')}
          />
        </div>
        <div className="filter-chips" role="radiogroup" aria-label={t('registry:filterAll')}>
          <button
            type="button"
            className={`chip ${selectedTier === 'all' ? 'active' : ''}`}
            onClick={() => setSelectedTier('all')}
          >
            {t('registry:filterAll')}
          </button>
          <button
            type="button"
            className={`chip ${selectedTier === 'official' ? 'active' : ''}`}
            onClick={() => setSelectedTier('official')}
          >
            {t('registry:filterOfficial')}
          </button>
          <button
            type="button"
            className={`chip ${selectedTier === 'verified_partner' ? 'active' : ''}`}
            onClick={() => setSelectedTier('verified_partner')}
          >
            {t('registry:filterVerifiedPartner')}
          </button>
          <button
            type="button"
            className={`chip ${selectedTier === 'community' ? 'active' : ''}`}
            onClick={() => setSelectedTier('community')}
          >
            {t('registry:filterCommunity')}
          </button>
          <button
            type="button"
            className={`chip chip-danger ${selectedTier === 'revoked' ? 'active' : ''}`}
            onClick={() => setSelectedTier('revoked')}
          >
            {t('registry:filterRevoked')}
          </button>
        </div>
      </div>

      {/* Extensions Grid */}
      {filteredExtensions.length === 0 ? (
        <p className="empty-message">{t('registry:emptySearch')}</p>
      ) : (
        <div className="marketplace-grid">
          {filteredExtensions.map((ext) => {
            const isRevoked = ext.reviewLevel === 'revoked';
            const isOfficial = ext.publisher.trustTier === 'official';
            const isPartner = ext.publisher.trustTier === 'verified_partner';

            return (
              <article
                key={ext.id}
                className={`marketplace-card ${isRevoked ? 'card-revoked' : ''}`}
              >
                <div className="card-top">
                  <div className="card-badge-row">
                    <span className="badge badge-type">
                      <FileCode size={12} aria-hidden="true" />
                      {t(ext.extensionType === 'mcp' ? 'registry:cardTypeMcp' : 'registry:cardTypeSkill')}
                    </span>
                    {isRevoked ? (
                      <span className="badge badge-danger">
                        <AlertTriangle size={12} aria-hidden="true" />
                        {t('registry:reviewRevoked')}
                      </span>
                    ) : isOfficial ? (
                      <span className="badge badge-official">
                        <ShieldCheck size={12} aria-hidden="true" />
                        {t('registry:tierOfficial')}
                      </span>
                    ) : isPartner ? (
                      <span className="badge badge-partner">
                        <CheckCircle2 size={12} aria-hidden="true" />
                        {t('registry:tierVerifiedPartner')}
                      </span>
                    ) : (
                      <span className="badge badge-community">
                        {t('registry:tierCommunity')}
                      </span>
                    )}
                  </div>
                  <span className="card-version mono">
                    {t('registry:cardVersion', { version: ext.version })}
                  </span>
                </div>

                <h3 className="card-title">{ext.displayName}</h3>
                <p className="card-package mono">{ext.name}</p>
                <p className="card-description">{ext.description}</p>

                {/* Key Fingerprint & Publisher */}
                <div className="card-meta">
                  <span className="meta-item">
                    <Key size={11} aria-hidden="true" />
                    <span className="mono">{`${ext.publisher.publicKeyFingerprint.slice(0, 18)}...`}</span>
                  </span>
                  <span className="meta-item">
                    {t('registry:cardPublisher', { name: ext.publisher.displayName })}
                  </span>
                </div>

                {/* Permissions Preview */}
                <div className="card-permissions">
                  {ext.permissions.map((p, idx) => (
                    <span
                      key={idx}
                      className={`perm-pill ${p.critical ? 'perm-critical' : ''}`}
                      title={p.description}
                    >
                      {p.target}
                    </span>
                  ))}
                </div>

                <div className="card-footer">
                  <button
                    type="button"
                    className={`button ${isRevoked ? 'button-danger' : 'button-secondary'}`}
                    onClick={() => handleOpenInspect(ext)}
                  >
                    {t('registry:inspectAndInstall')}
                  </button>
                </div>
              </article>
            );
          })}
        </div>
      )}

      {/* Permission Analysis & Security Inspection Modal */}
      {inspectingExtension && (
        <div
          className="modal-backdrop"
          role="dialog"
          aria-modal="true"
          aria-labelledby="modal-title"
        >
          <div className="modal-card">
            <header className="modal-header">
              <div>
                <span className="kicker">{t('registry:modalKicker')}</span>
                <h3 id="modal-title" className="modal-title">
                  {inspectingExtension.displayName}
                </h3>
              </div>
              <button
                type="button"
                className="icon-button"
                onClick={handleCloseInspect}
                aria-label={t('registry:modalClose')}
              >
                <X size={16} aria-hidden="true" />
              </button>
            </header>

            <div className="modal-body form-stack">
              {/* Target Agent Selector */}
              <div className="setting-row">
                <div>
                  <label className="setting-label" htmlFor="registry-target-agent">
                    {t('registry:targetAgentLabel')}
                  </label>
                  <p className="field-help">{t('registry:targetAgentHelp')}</p>
                </div>
                <span className="select-control">
                  <select
                    id="registry-target-agent"
                    value={targetAgent}
                    onChange={(e) => setTargetAgent(e.target.value as AgentId)}
                  >
                    {(['codex', 'claude', 'gemini', 'opencode'] as const).map((agent) => (
                      <option key={agent} value={agent}>
                        {t(`agents:${agent}`)}
                      </option>
                    ))}
                  </select>
                </span>
              </div>

              {/* Publisher & Cryptographic Signature Box */}
              <div className="sheet publisher-box">
                <h4 className="sheet-title">{t('registry:publisherSection')}</h4>
                <div className="info-grid">
                  <div>
                    <span className="info-label">{t('registry:publisherName')}</span>
                    <span className="info-val">{inspectingExtension.publisher.displayName}</span>
                  </div>
                  <div>
                    <span className="info-label">{t('registry:publisherKey')}</span>
                    <span className="info-val mono">
                      {inspectingExtension.publisher.publicKeyFingerprint}
                    </span>
                  </div>
                  <div>
                    <span className="info-label">{t('registry:publisherStatus')}</span>
                    <span className="info-val">
                      {signatureResult?.status === 'valid' ? (
                        <span className="badge badge-official">
                          <CheckCircle2 size={12} aria-hidden="true" />
                          {t('registry:sigValid')}
                        </span>
                      ) : signatureResult?.status === 'revoked' ? (
                        <span className="badge badge-danger">
                          <AlertTriangle size={12} aria-hidden="true" />
                          {t('registry:sigRevoked')}
                        </span>
                      ) : (
                        <span className="badge">{t('registry:sigUnsigned')}</span>
                      )}
                    </span>
                  </div>
                  <div>
                    <span className="info-label">{t('registry:shaDigest')}</span>
                    <span className="info-val mono">
                      {`${inspectingExtension.manifestSha256.slice(0, 24)}...`}
                    </span>
                  </div>
                </div>
              </div>

              {/* Revocation Warning if Revoked */}
              {permissionAnalysis?.isRevoked && (
                <div className="callout callout-danger" role="alert">
                  <div className="callout-header">
                    <ShieldAlert size={16} aria-hidden="true" />
                    <strong>{t('registry:revokedAlertTitle')}</strong>
                  </div>
                  <p>{permissionAnalysis.revocationReason || t('registry:revokedAlertBody')}</p>
                </div>
              )}

              {/* Privilege Escalation Warning */}
              {permissionAnalysis?.hasPrivilegeEscalation && !permissionAnalysis.isRevoked && (
                <div className="callout callout-warning" role="alert">
                  <div className="callout-header">
                    <AlertTriangle size={16} aria-hidden="true" />
                    <strong>{t('registry:escalationAlertTitle')}</strong>
                  </div>
                  <p>{t('registry:escalationAlertBody')}</p>
                  <label className="checkbox-row" style={{ marginTop: '8px' }}>
                    <input
                      type="checkbox"
                      checked={escalationConfirmed}
                      onChange={(e) => setEscalationConfirmed(e.target.checked)}
                    />
                    <span>{t('registry:escalationConfirmCheck')}</span>
                  </label>
                </div>
              )}

              {/* Declared Permissions Breakdown */}
              <div className="sheet permissions-box">
                <h4 className="sheet-title">{t('registry:permissionsSection')}</h4>
                <p className="field-help">{t('registry:permissionsHelp')}</p>
                <div className="perm-table">
                  {permissionAnalysis?.deltas.map((delta, i) => (
                    <div key={i} className="perm-row">
                      <div className="perm-col-info">
                        <strong>{delta.permission.description}</strong>
                        <span className="perm-target mono">{delta.permission.target}</span>
                      </div>
                      <div className="perm-col-badges">
                        {delta.isPrivilegeEscalation && (
                          <span className="badge badge-danger">
                            {t('registry:permCritical')}
                          </span>
                        )}
                        <span className="badge badge-subtle">
                          {delta.change === 'added'
                            ? t('registry:permAdded')
                            : t('registry:permUnchanged')}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Actions */}
              <div className="modal-actions" style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', justifyContent: 'flex-end' }}>
                <button
                  type="button"
                  className="button button-secondary"
                  onClick={handleCloseInspect}
                >
                  {t('registry:modalClose')}
                </button>
                <button
                  type="button"
                  className="button"
                  disabled={
                    permissionAnalysis?.isRevoked ||
                    !escalationConfirmed ||
                    isAnalyzing ||
                    flow.busy !== null
                  }
                  onClick={handleInstall}
                >
                  {permissionAnalysis?.isRevoked
                    ? t('registry:installBlocked')
                    : isAnalyzing
                    ? t('registry:installingAction')
                    : t('registry:installAction')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
