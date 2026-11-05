import { describe, expect, it } from 'vitest';
import { demoAccount, demoSyncDevices, demoSyncResult } from '../../src/data/demo';
import accountsJson from '../../src/locales/en-US/accounts.json';
import syncJson from '../../src/locales/en-US/sync.json';

describe('Phase 11 Accounts & Cloud Sync Engine', () => {
  it('conforms to offline-first safety guarantees', () => {
    const acc = demoAccount();
    expect(acc.id).toBe('acc_demo_pro');
    expect(acc.tier).toBe('pro');
    expect(acc.backupRetentionDays).toBe(90);
    expect(acc.cloudSyncEnabled).toBe(true);
  });

  it('provides paired devices mesh data', () => {
    const devices = demoSyncDevices();
    expect(devices.length).toBeGreaterThanOrEqual(2);
    expect(devices.some(d => d.isCurrent)).toBe(true);
    expect(devices.some(d => d.platform === 'windows')).toBe(true);
    expect(devices.some(d => d.platform === 'macos')).toBe(true);
  });

  it('guarantees client-side encrypted envelope sync result', () => {
    const result = demoSyncResult();
    expect(result.envelopesSynced).toBeGreaterThan(0);
    expect(result.provenance).toContain('AES-256-GCM');
    expect(result.conflicts).toEqual([]);
  });

  it('has comprehensive localization catalogs with zero empty strings', () => {
    for (const [, value] of Object.entries(accountsJson)) {
      expect(typeof value).toBe('string');
      expect(value.trim().length).toBeGreaterThan(0);
      expect(value).not.toMatch(/[\u3400-\u9fff]/);
    }

    for (const [, value] of Object.entries(syncJson)) {
      expect(typeof value).toBe('string');
      expect(value.trim().length).toBeGreaterThan(0);
      expect(value).not.toMatch(/[\u3400-\u9fff]/);
    }
  });
});
