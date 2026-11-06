import { describe, expect, it } from 'vitest';
import { demoGovernanceState } from '../../src/data/demo';
import { resources } from '../../src/app/resources';

describe('Team & Organization Control Plane (Phase 13 / v1.0.0)', () => {
  it('seeds a cryptographically verified organization policy with 100% baseline compliance', () => {
    const state = demoGovernanceState();
    expect(state.policy.orgId).toBe('org_nexus_enterprise');
    expect(state.policy.signatureVerified).toBe(true);
    expect(state.policy.policySignatureEd25519).toMatch(/^ed25519:/);
    expect(state.policy.policyDigestSha256).toMatch(/^sha256:/);
    expect(state.policy.userRole).toBe('owner');
    expect(state.policy.canMutatePolicy).toBe(true);
    expect(state.policy.approvedAgents).toEqual(['claude', 'codex', 'gemini', 'opencode']);
    expect(state.compliance.overallStatus).toBe('compliant');
    expect(state.compliance.complianceScore).toBe(100);
    expect(state.compliance.checks.length).toBe(5);
  });

  it('defines an explicit 4-level deterministic policy precedence chain with emergency break-glass at the top', () => {
    const state = demoGovernanceState();
    expect(state.policy.precedenceChain).toHaveLength(4);
    expect(state.policy.precedenceChain[0]).toContain('Emergency Local Break-Glass');
    expect(state.policy.precedenceChain[1]).toContain('Cryptographically Signed Organization Policy');
    expect(state.policy.precedenceChain[2]).toContain('Team Workspace Template');
    expect(state.policy.precedenceChain[3]).toContain('Local Developer Preferences');
  });

  it('ships 3 standardized enterprise workspace templates with instructions and extension requirements', () => {
    const state = demoGovernanceState();
    expect(state.templates.map(t => t.id)).toEqual([
      'tpl_rust_systems',
      'tpl_fullstack_ts',
      'tpl_sec_audit',
    ]);
    for (const tpl of state.templates) {
      expect(tpl.orgId).toBe('org_nexus_enterprise');
      expect(tpl.requiredExtensions.length).toBeGreaterThan(0);
      expect(tpl.instructionTemplate.length).toBeGreaterThan(20);
    }
  });

  it('enforces a strict zero-knowledge scoped health visibility boundary', () => {
    const state = demoGovernanceState();
    expect(state.visibilityBoundary.sharedFields.length).toBeGreaterThanOrEqual(4);
    expect(state.visibilityBoundary.neverSharedFields.length).toBeGreaterThanOrEqual(4);
    expect(state.visibilityBoundary.neverSharedFields.join(' ')).toContain('API Keys');
    expect(state.visibilityBoundary.neverSharedFields.join(' ')).toContain('Conversation Prompts');
    expect(state.visibilityBoundary.neverSharedFields.join(' ')).toContain('Local Source Code');
  });

  it('includes complete English localization keys for the governance namespace', () => {
    const gov = resources['en-US'].governance as Record<string, string>;
    expect(gov.title).toBe('Team & Organization Control Plane');
    expect(gov.role_owner).toBe('Organization Owner');
    expect(gov.role_admin).toBe('Security Admin');
    expect(gov.role_member).toBe('Developer Member');
    expect(gov.role_auditor).toBe('Compliance Auditor');
    expect(gov.activateBreakGlassButton).toBeTruthy();
  });
});
