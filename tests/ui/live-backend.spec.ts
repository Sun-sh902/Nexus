import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { readFileSync } from 'node:fs';
import { resources } from '../../src/app/resources';

// Live-mode UI against IPC responses produced by the real Rust backend from an isolated fake
// home (regenerate: `cargo test export_live_ui_fixture -- --ignored`). The IPC bridge is the
// only thing simulated; no fixture data from src/data/fixtures.ts may appear.
const fixture = (name: string) => JSON.parse(readFileSync(`tests/fixtures/live/${name}.json`, 'utf8'));

async function startLive(page: Page, locale: 'en-US', route: string, options: { deny?: boolean; failScan?: boolean; failModels?: boolean } = {}) {
  const data = { overview: fixture('overview'), after: fixture('overview-after'), models: fixture('models'), plan: fixture('plan'), result: fixture('result') };
  for (const overview of [data.overview, data.after]) for (const agent of overview.agents) agent.health.validUntil = new Date(Date.now() + 600_000).toISOString();
  await page.addInitScript(({ locale, data, options }) => {
    localStorage.setItem('nexus.preferences.v1', JSON.stringify({ version: 1, theme: 'light' }));
    const calls: { cmd: string; args: unknown }[] = [];
    let confirmed = false;
    let pruned = false;
    (window as unknown as { __calls: typeof calls }).__calls = calls;
    (window as unknown as { __failModels?: boolean }).__failModels = Boolean(options.failModels);
    (window as unknown as { isTauri: boolean }).isTauri = true;
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
      transformCallback: () => 0,
      invoke: async (cmd: string, args: Record<string, unknown>) => {
        calls.push({ cmd, args });
        switch (cmd) {
          case 'app_metadata': return { contractVersion: 1, version: '0.7.0', preferredLocale: locale };
          case 'apply_app_preferences': return { contractVersion: 1, requestId: (args.request as { requestId: string }).requestId, result: 'applied' };
          case 'nexus_set_locale': return null;
          case 'nexus_overview':
            if (options.failScan || (window as unknown as { __failAfter?: boolean }).__failAfter) throw { code: 'store.open_failed', params: [], retryable: true };
            return confirmed ? data.after : data.overview;
          case 'nexus_models':
            if ((window as unknown as { __failModels?: boolean }).__failModels) throw { code: 'store.open_failed', params: [], retryable: true };
            return data.models;
          case 'nexus_agent_history':
            return {
              backups: [{ id: 'bkp_1', operationId: 'op_1', agentId: 'claude', targetId: 'claude.settings', targetDisplay: '~/.claude/settings.json', sha256: 'abcdef0123456789', size: 128, createdAt: '2026-11-03T07:00:00Z' }],
              operations: [{ id: 'op_1', action: 'applyModel', agentId: 'claude', status: 'succeeded', startedAt: '2026-11-03T07:00:00Z', finishedAt: '2026-11-03T07:00:01Z', errorCode: null, summary: {}, steps: ['backup', 'write', 'verify'] }],
              health: [{ readiness: 'attention', ruleSet: 'health.v1', issueCodes: ['assignmentDrift'], evaluatedAt: '2026-11-03T07:05:00Z' }],
            };
          case 'nexus_prepare_action': return data.plan;
          case 'nexus_discard_plan': return null;
          case 'nexus_delete_profile': return null;
          case 'nexus_storage_status':
            return pruned
              ? { dataDir: '~/Library/Application Support/dev.nexus.app', dbBytes: 49_152, backupCount: 1, backupBytes: 512, operationCount: 0, unresolvedCount: 0, healthRowCount: 4 }
              : { dataDir: '~/Library/Application Support/dev.nexus.app', dbBytes: 65_536, backupCount: 3, backupBytes: 2048, operationCount: 5, unresolvedCount: 0, healthRowCount: 16 };
          case 'nexus_prune_history':
            pruned = true;
            return { prunedOperations: 5, prunedHealthRows: 12, prunedBackups: 2 };
          case 'nexus_diagnostic_preview':
            return { filename: 'nexus-diagnostics.json', json: JSON.stringify({ schemaVersion: 1, appVersion: '0.7.0', system: { dataDir: '~/Library/Application Support/dev.nexus.app' } }, null, 2) };
          case 'nexus_check_provider':
            return { providerId: (args as { providerId: string }).providerId, status: 'connected', httpStatus: 200, latencyMs: 35, message: 'Endpoint reachable and verified', modelsDiscovered: ['gpt-4o'] };
          case 'nexus_confirm_plan':
            if (options.deny) throw { code: 'confirmation.denied', params: [], retryable: false };
            confirmed = true;
            return data.result;
          default: throw { code: 'ipc.invalid_input', params: [], retryable: false };
        }
      },
    };
  }, { locale, data, options });
  await page.goto(`/#/${route}`);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);
}

for (const locale of ['en-US'] as const) {
  const m = resources[locale];
  test(`${locale}: live Home shows only backend data and labels its provenance`, async ({ page }) => {
    await startLive(page, locale, 'home');
    await expect(page.locator('.demo-register')).toContainText(m.common.liveShort);
    await expect(page.locator('.demo-register')).not.toContainText(m.common.fixtureShort);
    await expect(page.locator('h1')).toContainText(m.home['headlineLead_other'].replace('{{ready}}', '0').replace('{{total}}', '4'));
    for (const name of ['OpenAI Codex', 'Claude Code', 'Gemini CLI', 'OpenCode']) await expect(page.locator('.agent-ledger')).toContainText(name);
    // No Phase 1 fixture values leak into a live session.
    await expect(page.locator('body')).not.toContainText('0.0.0-demo');
    await expect(page.locator('body')).not.toContainText('demo-coding');
    await expect(page.locator('.agent-ledger')).toContainText('0.164.0');
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(results.violations.map(v => v.id)).toEqual([]);
  });

  test(`${locale}: expired evidence no longer appears ready`, async ({ page }) => {
    await page.clock.install();
    await startLive(page, locale, 'home');
    await expect(page.locator('.agent-ledger')).toBeVisible();
    await page.clock.fastForward(601_000);
    await expect(page.locator('main')).toContainText(m.common.scanFailed);
  });

  test(`${locale}: a failed rescan is visible after a successful scan`, async ({ page }) => {
    await startLive(page, locale, 'home');
    await expect(page.locator('.agent-ledger')).toBeVisible();
    await page.evaluate(() => { (window as unknown as { __failAfter: boolean }).__failAfter = true; });
    await page.getByRole('button', { name: m.common.rescan }).click();
    await expect(page.locator('main')).toContainText(m.errors['store.open_failed']);
  });

  test(`${locale}: model loading failure is visible and can be retried`, async ({ page }) => {
    await startLive(page, locale, 'models', { failModels: true });
    await expect(page.locator('main')).toContainText(m.errors['store.open_failed']);
    const retry = page.locator('.notice').getByRole('button', { name: m.common.retryScan });
    await expect(retry).toBeVisible();
    await page.evaluate(() => { (window as unknown as { __failModels: boolean }).__failModels = false; });
    await retry.click();
    await expect(page.locator('.chain-sheet')).toBeVisible();
  });

  test(`${locale}: finding → plan → native confirmation → verified result`, async ({ page }) => {
    await startLive(page, locale, 'agents/codex');
    await page.getByRole('tab', { name: m.agents.diagnostics }).click();
    const finding = page.locator('.finding').first();
    await expect(finding).toContainText(m.findings['mcpCommandMissing.title']);
    await finding.getByRole('button', { name: m.actions['repair.setMcpEnabled'] }).click();
    const plan = page.locator('.plan-sheet');
    await expect(plan).toContainText(m.actions['kind.setMcpEnabled']);
    await expect(plan).toContainText('mcp_servers.files.enabled');
    await expect(plan.locator('.plan-diff')).toHaveCount(0);
    await expect(plan).toContainText(m.actions['backup.encrypted']);
    await plan.getByRole('button', { name: m.actions.confirm }).click();
    const result = page.locator('.plan-sheet').filter({ hasText: m.actions['status.succeeded'] });
    await expect(result).toContainText(m.actions['verification.passed']);
    // The renderer sent only the plan id and digest; Rust owns the confirmation.
    const calls = await page.evaluate(() => (window as unknown as { __calls: { cmd: string; args: Record<string, unknown> }[] }).__calls.filter(c => c.cmd === 'nexus_confirm_plan'));
    expect(Object.keys(calls[0].args).sort()).toEqual(['digest', 'planId']);
    await expect(page.locator('.finding')).toHaveCount(0);
  });

  test(`${locale}: denied confirmation changes nothing and says so`, async ({ page }) => {
    await startLive(page, locale, 'agents/codex', { deny: true });
    await page.getByRole('tab', { name: m.agents.diagnostics }).click();
    await page.locator('.finding').first().getByRole('button').first().click();
    await page.getByRole('button', { name: m.actions.confirm }).click();
    await expect(page.locator('.action-panel')).toContainText(m.errors['confirmation.denied']);
    await expect(page.locator('.action-panel')).toContainText(m.actions.deniedHelp);
  });

  test(`${locale}: scan failure shows an error state, never demo data`, async ({ page }) => {
    await startLive(page, locale, 'home', { failScan: true });
    await expect(page.locator('main')).toContainText(m.common.scanFailed);
    await expect(page.locator('main')).toContainText(m.errors['store.open_failed']);
    await expect(page.locator('body')).not.toContainText('0.0.0-demo');
  });

  test(`${locale}: model precedence, installations and capability limits are explained`, async ({ page }) => {
    await startLive(page, locale, 'agents/claude');
    await expect(page.locator('.deck')).toContainText(m.findings['assignmentDrift.title']);
    await page.getByRole('tab', { name: m.agents.modelTab }).click();
    await expect(page.locator('.layer-list')).toContainText(m.agents['liveSource.userConfig']);
    await expect(page.locator('.layer-list')).toContainText('haiku');
    await page.getByRole('tab', { name: m.agents.configuration }).click();
    await expect(page.locator('main')).toContainText(m.agents['installSource.nativeInstaller']);
    await expect(page.locator('main')).toContainText(m.capability['capability.nativeUninstallManual']);
    await expect(page.locator('main')).toContainText(m.agents.operationsHistoryTitle);
    await expect(page.locator('main')).toContainText(m.actions['kind.applyModel']);
    await expect(page.locator('main')).toContainText(m.agents.healthHistoryTitle);
    await page.goto('/#/agents/opencode');
    await page.getByRole('tab', { name: m.agents.modelTab }).click();
    await expect(page.locator('main')).toContainText(m.capability['capability.opencodeAmbiguousConfig']);
  });

  test(`${locale}: reviewed local MCP recipes prepare typed plans and respect Agent boundaries`, async ({ page }) => {
    await startLive(page, locale, 'agents/codex');
    await page.getByRole('tab', { name: m.agents.extensions }).click();
    await page.locator('#mcp-reviewed-recipe').selectOption('mcp.filesystem');
    await page.locator('#mcp-reviewed-name').fill('project_fs');
    await page.locator('#mcp-reviewed-dir').fill('~/Projects/demo');
    await page.locator('[aria-labelledby="add-reviewed-mcp-title"]').getByRole('button', { name: m.extensions.liveAddPreview }).click();
    const prepCalls = await page.evaluate(() => (window as unknown as { __calls: { cmd: string; args: { request: Record<string, unknown> } }[] }).__calls.filter(c => c.cmd === 'nexus_prepare_action'));
    expect(prepCalls[0].args.request).toEqual({
      kind: 'addReviewedMcp',
      agentId: 'codex',
      name: 'project_fs',
      recipeId: 'mcp.filesystem',
      directory: '~/Projects/demo',
    });
    await page.goto('/#/agents/claude');
    await page.getByRole('tab', { name: m.agents.extensions }).click();
    await expect(page.locator('[aria-labelledby="add-reviewed-mcp-title"]')).toContainText(m.capability['capability.claudeMcpAgentOwned']);
  });

  test(`${locale}: live Settings prunes local history and previews redacted diagnostics before export`, async ({ page }) => {
    await startLive(page, locale, 'settings');
    await expect(page.locator('main')).toContainText(m.settings.privacy);
    await expect(page.locator('main')).toContainText('~/Library/Application Support/dev.nexus.app');
    await page.getByRole('button', { name: m.settings.pruneButton }).click();
    await expect(page.locator('main')).toContainText('Pruned 5 operations');
    await page.getByRole('button', { name: m.settings.previewDiagnostics }).click();
    await expect(page.locator('pre.plan-diff')).toContainText('"schemaVersion": 1');
    await page.getByRole('button', { name: m.settings.downloadDiagnostics }).click();
    await expect(page.locator('main')).toContainText('Saved nexus-diagnostics.json.');
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(results.violations.map(v => v.id)).toEqual([]);
  });
}
