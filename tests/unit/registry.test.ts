import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { demoRegistryExtensions, demoRegistrySyncSummary } from '../../src/data/demo';

describe('Phase 12 Verified Extension Registry', () => {
  it('provides a curated catalog of signed MCP and Skill extensions', () => {
    const extensions = demoRegistryExtensions();
    expect(extensions.length).toBeGreaterThanOrEqual(6);

    const memory = extensions.find((e) => e.id === 'mcp-memory');
    expect(memory).toBeDefined();
    expect(memory?.reviewLevel).toBe('verified_official');
    expect(memory?.signatureStatus).toBe('valid');
    expect(memory?.publisher.trustTier).toBe('official');

    const github = extensions.find((e) => e.id === 'mcp-github');
    expect(github).toBeDefined();
    expect(github?.publisher.trustTier).toBe('verified_partner');
    expect(github?.permissions.some((p) => p.kind === 'network_access')).toBe(true);

    const gitSkill = extensions.find((e) => e.id === 'skill-git-workflow');
    expect(gitSkill).toBeDefined();
    expect(gitSkill?.extensionType).toBe('skill');
    expect(gitSkill?.reviewLevel).toBe('community_reviewed');
  });

  it('detects and flags revoked artifacts with cryptographic killswitch', () => {
    const extensions = demoRegistryExtensions();
    const rogue = extensions.find((e) => e.id === 'mcp-rogue-pipe');
    expect(rogue).toBeDefined();
    expect(rogue?.reviewLevel).toBe('revoked');
    expect(rogue?.signatureStatus).toBe('revoked');
    expect(rogue?.revocationReason).toContain('CVE-2026-NEXUS-01');
  });

  it('verifies offline-first sync summary with local cache validation', () => {
    const summary = demoRegistrySyncSummary();
    expect(summary.offlineCached).toBe(true);
    expect(summary.totalExtensions).toBeGreaterThan(0);
    expect(summary.revocationCount).toBeGreaterThan(0);
    expect(summary.source).toContain('Offline Cache Verified');
  });

  it('ensures all registry strings are complete and non-empty in en-US catalog', () => {
    const raw = readFileSync('src/locales/en-US/registry.json', 'utf8');
    const catalog = JSON.parse(raw);
    expect(Object.keys(catalog).length).toBeGreaterThan(25);
    for (const [, value] of Object.entries(catalog)) {
      expect(typeof value).toBe('string');
      expect((value as string).trim().length).toBeGreaterThan(0);
      expect(value).not.toMatch(/[\u3400-\u9fff]/); // English only
    }
  });
});
