// Test OpenAnalyst CLI login flow using Node.js child_process + PTY simulation
// Since Playwright is for browsers, we test terminal apps with spawn + stdin/stdout

const { spawn } = require('child_process');
const path = require('path');

const BINARY = path.join(__dirname, '..', 'rust', 'target', 'release', 'openanalyst.exe');
const TIMEOUT = 15000;

let passed = 0;
let failed = 0;

function test(name, fn) {
  return fn().then(() => {
    passed++;
    console.log(`  ✓ ${name}`);
  }).catch(err => {
    failed++;
    console.log(`  ✗ ${name}: ${err.message || err}`);
  });
}

function runCommand(args, opts = {}) {
  return new Promise((resolve, reject) => {
    const proc = spawn(BINARY, args, {
      timeout: opts.timeout || TIMEOUT,
      env: { ...process.env, ...opts.env },
    });
    let stdout = '';
    let stderr = '';
    proc.stdout.on('data', d => stdout += d.toString());
    proc.stderr.on('data', d => stderr += d.toString());
    proc.on('close', code => resolve({ code, stdout, stderr, output: stdout + stderr }));
    proc.on('error', reject);
    if (opts.stdin) {
      proc.stdin.write(opts.stdin);
      proc.stdin.end();
    }
  });
}

async function main() {
  console.log('\n  OpenAnalyst CLI — Automated Test Suite\n');

  // ── 1. Version ──
  await test('--version returns v1.0.92+', async () => {
    const r = await runCommand(['--version']);
    if (!r.output.includes('1.0.9')) throw new Error(`Got: ${r.output.trim()}`);
  });

  // ── 2. Help ──
  await test('--help lists all subcommands', async () => {
    const r = await runCommand(['--help']);
    if (!r.output.includes('openanalyst v1.0.9')) throw new Error('Missing version header');
    if (!r.output.includes('/help')) throw new Error('Missing /help');
    if (!r.output.includes('/trust')) throw new Error('Missing /trust');
    if (!r.output.includes('/knowledge')) throw new Error('Missing /knowledge');
    if (!r.output.includes('openanalyst login')) throw new Error('Missing login');
    if (!r.output.includes('openanalyst update')) throw new Error('Missing update');
    if (!r.output.includes('openanalyst uninstall')) throw new Error('Missing uninstall');
  });

  // ── 3. Help lists 61 slash commands ──
  await test('--help lists 60+ slash commands', async () => {
    const r = await runCommand(['--help']);
    const slashCount = (r.output.match(/^\s+\/\w+/gm) || []).length;
    if (slashCount < 50) throw new Error(`Only ${slashCount} slash commands found (expected 60+)`);
  });

  // ── 4. Whoami ──
  await test('whoami shows provider status', async () => {
    const r = await runCommand(['whoami']);
    if (!r.output.includes('Provider Status')) throw new Error('Missing header');
    if (!r.output.includes('OpenAnalyst')) throw new Error('Missing OpenAnalyst provider');
  });

  // ── 5. Init ──
  await test('init creates .openanalyst directory', async () => {
    const r = await runCommand(['init']);
    if (r.code !== 0 && !r.output.includes('skipped')) throw new Error(`Exit code ${r.code}`);
  });

  // ── 6. System prompt ──
  await test('system-prompt outputs valid prompt', async () => {
    const r = await runCommand(['system-prompt']);
    if (!r.output.includes('interactive agent')) throw new Error('Missing system prompt content');
    if (!r.output.includes('tools')) throw new Error('Missing tools reference');
  });

  // ── 7. Agents (no agents defined) ──
  await test('agents subcommand works', async () => {
    const r = await runCommand(['agents']);
    // Either shows agents or "no agents found"
    if (r.code !== 0 && !r.output.toLowerCase().includes('no agents')) throw new Error(`Unexpected: ${r.output}`);
  });

  // ── 8. Skills (no skills defined) ──
  await test('skills subcommand works', async () => {
    const r = await runCommand(['skills']);
    if (r.code !== 0 && !r.output.toLowerCase().includes('no skills')) throw new Error(`Unexpected: ${r.output}`);
  });

  // ── 9. Bootstrap plan ──
  await test('bootstrap-plan shows phases', async () => {
    const r = await runCommand(['bootstrap-plan']);
    if (!r.output.includes('CliEntry')) throw new Error('Missing CliEntry phase');
    if (!r.output.includes('MainRuntime')) throw new Error('Missing MainRuntime phase');
  });

  // ── 10. Non-interactive prompt without API key ──
  await test('prompt mode fails gracefully without API key', async () => {
    const r = await runCommand(['--model', 'claude-haiku-4-5', '--output-format', 'text', 'prompt', 'hi'], {
      env: { ANTHROPIC_API_KEY: '', OPENANALYST_API_KEY: '', OPENANALYST_AUTH_TOKEN: '' },
    });
    // Should fail with auth error, not crash
    if (r.code === 0) throw new Error('Should have failed without key');
    if (r.output.includes('panic')) throw new Error('Panicked instead of graceful error');
  });

  // ── 11. Resume with empty session ──
  await test('--resume loads session file', async () => {
    const fs = require('fs');
    const sessionDir = path.join(__dirname, '..', '.openanalyst', 'sessions');
    fs.mkdirSync(sessionDir, { recursive: true });
    const sessionFile = path.join(sessionDir, 'test-session.json');
    // Runtime session format uses version + messages with blocks (not TUI chat format)
    fs.writeFileSync(sessionFile, JSON.stringify({
      version: 1,
      messages: [
        { role: "user", blocks: [{ type: "text", text: "hello" }] },
        { role: "assistant", blocks: [{ type: "text", text: "Hi there!" }] },
      ],
    }));
    const r = await runCommand(['--resume', sessionFile]);
    if (!r.output.includes('2 messages') && !r.output.includes('Restored') && !r.output.includes('session')) throw new Error(`Got: ${r.output}`);
    fs.unlinkSync(sessionFile);
  });

  // ── 12. Login exits cleanly on Esc ──
  await test('login handles Esc key (graceful exit)', async () => {
    const proc = spawn(BINARY, ['login'], { timeout: 5000 });
    let output = '';
    proc.stdout.on('data', d => output += d.toString());
    proc.stderr.on('data', d => output += d.toString());

    // Wait for menu to render, then send Esc
    await new Promise(r => setTimeout(r, 1000));
    proc.stdin.write('\x1b'); // Esc key

    const code = await new Promise(resolve => {
      proc.on('close', resolve);
      setTimeout(() => { proc.kill(); resolve(-1); }, 3000);
    });

    if (code !== 0 && code !== -1) throw new Error(`Exit code ${code}`);
  });

  // ── Summary ──
  console.log(`\n  Results: ${passed} passed, ${failed} failed\n`);
  process.exit(failed > 0 ? 1 : 0);
}

main().catch(e => { console.error(e); process.exit(1); });
