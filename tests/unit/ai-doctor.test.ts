import { describe, expect, it } from 'vitest';
import { demoAiDoctor } from '../../src/data/demo';

describe('AI Doctor & Health Radar', () => {
  it('generates grounded 7-stage radar telemetry and expert reasoning for healthy agents', () => {
    const consultation = demoAiDoctor('claude');
    expect(consultation.agentId).toBe('claude');
    expect(consultation.overallSeverity).toBe('healthy');
    expect(consultation.confidenceScore).toBeGreaterThanOrEqual(0.95);
    expect(consultation.radarStages).toHaveLength(7);
    expect(consultation.radarStages.map(s => s.id)).toEqual([
      'runtime',
      'toolchain',
      'config',
      'auth',
      'model',
      'extensions',
      'network',
    ]);
    expect(consultation.redactionNotice).toContain('100% of sensitive paths');
    expect(consultation.disclaimer).toContain('allowlisted native actions');
  });

  it('ranks probable causes and suggests allowlisted safe repair for drifting agents', () => {
    const consultation = demoAiDoctor('codex');
    expect(consultation.agentId).toBe('codex');
    expect(consultation.overallSeverity).toBe('advisory');
    expect(consultation.rankedCauses.length).toBeGreaterThan(0);
    expect(consultation.rankedCauses[0].category).toBe('ModelAssignment');
    expect(consultation.recommendations).toHaveLength(1);
    expect(consultation.recommendations[0].action).toEqual({
      kind: 'applyModel',
      agentId: 'codex',
    });
  });
});
