import { describe, it, expect } from 'vitest';
import { demoSessions, demoUsageInsights } from '../../src/data/demo';
import { resources } from '../../src/app/resources';

describe('Phase 10: Session Continuity & Contextual Usage Insights', () => {
  it('supplies complete and grounded session continuity telemetry in demo fixtures', () => {
    const sessions = demoSessions();
    expect(sessions.length).toBeGreaterThanOrEqual(2);

    for (const session of sessions) {
      expect(session.id).toMatch(/^demo_sess_/);
      expect(['claude', 'codex', 'gemini', 'opencode']).toContain(session.agentId);
      expect(session.totalTokens).toBe(session.inputTokens + session.outputTokens);
      expect(session.canResume).toBe(true);
      expect(session.resumeCommandPreview).toContain(session.agentId === 'opencode' ? 'opencode' : session.agentId);
      expect(session.estimatedCostUsd).toMatch(/^\$\d+\.\d{2}$/);
      expect(session.costProvenance).toBeDefined();
    }
  });

  it('aggregates usage insights with correct arithmetic and transparent provenance', () => {
    const insights = demoUsageInsights();
    expect(insights.totalTokens).toBe(insights.totalInputTokens + insights.totalOutputTokens);
    expect(insights.totalSessionsCount).toBeGreaterThanOrEqual(1);
    expect(insights.byAgent.length).toBeGreaterThanOrEqual(1);
    expect(insights.byModel.length).toBeGreaterThanOrEqual(1);
    expect(insights.byWorkspace.length).toBeGreaterThanOrEqual(1);

    // Sum of agent tokens matches total tokens
    const agentTokenSum = insights.byAgent.reduce((acc, a) => acc + a.totalTokens, 0);
    expect(agentTokenSum).toBe(insights.totalTokens);

    // Percentage of total is bounded [0, 100]
    for (const a of insights.byAgent) {
      expect(a.percentageOfTotal).toBeGreaterThanOrEqual(0);
      expect(a.percentageOfTotal).toBeLessThanOrEqual(100);
    }

    // Provenance notes exist and are localized keys
    expect(insights.provenanceNotes.length).toBeGreaterThanOrEqual(3);
    for (const note of insights.provenanceNotes) {
      expect(note).toMatch(/^usage\.provenance\./);
    }
  });

  it('provides complete en-US localization catalogs for sessions and usage', () => {
    const sessionsCatalog = resources['en-US'].sessions;
    expect(sessionsCatalog.title).toBe('Agent Sessions');
    expect(sessionsCatalog.resume).toBe('Resume');
    expect(sessionsCatalog.delete).toBe('Remove reference');

    const usageCatalog = resources['en-US'].usage;
    expect(usageCatalog.title).toBe('Contextual Usage & Cost Insights');
    expect(usageCatalog.totalTokens).toBe('Total Tokens');
    expect(usageCatalog.estimatedCost).toBe('Estimated Cost');
    expect(usageCatalog.provenanceTitle).toBe('Accounting Provenance & Privacy Guarantees');
  });
});
