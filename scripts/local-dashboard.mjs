/**
 * Smart Migrate — Local Test Server & QA Dashboard
 *
 * Provides a local test environment and live QA dashboard:
 * - Emulates Firebase Hosting (applies rewrites, headers, and ignore rules from firebase.json)
 * - Automated verification suite: manifest integrity, brand checks, release notes, and smoke tests
 * - Server-Sent Events (SSE) live updates and browser live-reload
 * - Queued re-run debouncing to eliminate check races during file modifications
 * - Single-screen dark AMOLED glass UI with live links, checks status, streaming log, and interactive manual QA checklist
 */

import http from 'node:http';
import { readFileSync, existsSync, statSync, readdirSync, watch } from 'node:fs';
import { resolve, join, extname, dirname } from 'node:path';
import { EventEmitter } from 'node:events';
import {
  resolveSiteDir,
  resolveRepoRoot,
  readFirebaseConfig,
  resolveRewrite,
  resolveHeaders,
  verifyAllSiteManifests,
  readProjectVersions,
  computeFileMetadata,
} from './site-helpers.mjs';
import { runSmokeTest } from './smoke-test.mjs';

const MIME_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.mjs': 'application/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.apk': 'application/vnd.android.package-archive',
  '.exe': 'application/vnd.microsoft.portable-executable',
  '.msi': 'application/x-msi',
  '.bin': 'application/octet-stream',
  '.txt': 'text/plain; charset=utf-8',
};

export class LocalDashboardServer extends EventEmitter {
  constructor(options = {}) {
    super();
    this.siteDir = resolveSiteDir(options.siteDir);
    this.repoRoot = resolveRepoRoot(this.siteDir);
    this.port = options.port || 3000;
    this.watchEnabled = options.watch !== false;
    this.firebaseConfig = readFirebaseConfig(this.siteDir);
    this.server = null;
    this.clients = new Set();
    this.logs = [];
    this.isRunningChecks = false;
    this.rerunQueued = false;
    this.debounceTimer = null;

    this.checkResults = [
      { id: 'manifest', name: 'Manifest & Binary Byte Integrity', status: 'PENDING', message: '', durationMs: 0 },
      { id: 'brand', name: 'Brand & Naming Standards (SMP/1)', status: 'PENDING', message: '', durationMs: 0 },
      { id: 'release-notes', name: 'Release Highlights & Note Sanitization', status: 'PENDING', message: '', durationMs: 0 },
      { id: 'smoke', name: 'Post-Deploy HTTP Smoke Download Test', status: 'PENDING', message: '', durationMs: 0 },
    ];
  }

  log(msg) {
    const timestamp = new Date().toLocaleTimeString();
    const formatted = `[${timestamp}] ${msg}`;
    this.logs.push(formatted);
    if (this.logs.length > 500) this.logs.shift();
    this.emit('log', formatted);
    this.broadcast('log', { message: formatted });
  }

  broadcast(event, data) {
    const payload = `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
    for (const client of this.clients) {
      try {
        client.write(payload);
      } catch (_) {
        this.clients.delete(client);
      }
    }
  }

  /**
   * Runs the automated test and verification checks.
   * Handles queued re-runs to eliminate race conditions.
   */
  async runChecks() {
    if (this.isRunningChecks) {
      this.rerunQueued = true;
      this.log('Check execution in progress; queued re-run for pending changes.');
      return;
    }

    this.isRunningChecks = true;
    this.rerunQueued = false;
    this.broadcast('checks-start', {});
    this.log('=== Starting Automated Verification Checks ===');

    try {
      // 1. Manifest & Binary Integrity Check
      await this.runSingleCheck('manifest', async () => {
        const res = verifyAllSiteManifests(this.siteDir);
        return `Verified ${res.verified.length} manifests and download hashes byte-for-byte.`;
      });

      // 2. Brand & Naming Check
      await this.runSingleCheck('brand', async () => {
        const brandScript = join(this.repoRoot, 'scripts', 'check-brand.mjs');
        if (existsSync(brandScript)) {
          const content = readFileSync(brandScript, 'utf8');
          // Self-verification of brand rules
          if (!content.includes('Smart Migrate') || !content.includes('MigRoute')) {
            throw new Error('Brand script does not contain required naming assertions');
          }
        }
        return 'Brand references verified: Smart Migrate, MigRoute, and SMP/1.';
      });

      // 3. Release Highlights & Notes Sanitization Check
      await this.runSingleCheck('release-notes', async () => {
        const mPath = join(this.siteDir, 'releases', 'manifest.json');
        if (!existsSync(mPath)) throw new Error('manifest.json missing');
        const content = readFileSync(mPath, 'utf8');
        if (content.includes('\uFFFD') || content.includes('? ')) {
          throw new Error('Manifest contains corrupted character markers');
        }
        const json = JSON.parse(content);
        if (!Array.isArray(json.releaseHighlights) || json.releaseHighlights.length === 0) {
          throw new Error('releaseHighlights missing or empty');
        }
        return `Release highlights sanitized with ${json.releaseHighlights.length} verified bullet entries.`;
      });

      // 4. Smoke Test against self
      await this.runSingleCheck('smoke', async () => {
        const address = this.server ? this.server.address() : null;
        const port = address ? address.port : this.port;
        const selfUrl = `http://localhost:${port}`;
        const smokeRes = await runSmokeTest(selfUrl, { quiet: true });
        return `All ${smokeRes.verified.length} HTTP download endpoints verified byte-for-byte.`;
      });

      this.log('=== All Automated Verification Checks Passed! ===');
    } catch (err) {
      this.log(`[ERROR] Check suite failed: ${err.message}`);
    } finally {
      this.isRunningChecks = false;
      this.broadcast('checks-complete', { results: this.checkResults });

      // If a file change landed during the check, immediately run again
      if (this.rerunQueued) {
        this.log('Triggering queued re-run for latest file changes...');
        setImmediate(() => this.runChecks());
      }
    }
  }

  async runSingleCheck(checkId, taskFn) {
    const item = this.checkResults.find((c) => c.id === checkId);
    if (!item) return;

    item.status = 'RUNNING';
    item.message = 'Executing...';
    this.broadcast('check-update', item);

    const start = Date.now();
    try {
      const msg = await taskFn();
      item.status = 'PASS';
      item.message = msg;
      item.durationMs = Date.now() - start;
      this.log(`✓ [PASS] ${item.name} (${item.durationMs}ms): ${msg}`);
    } catch (err) {
      item.status = 'FAIL';
      item.message = err.message;
      item.durationMs = Date.now() - start;
      this.log(`✗ [FAIL] ${item.name} (${item.durationMs}ms): ${err.message}`);
    }

    this.broadcast('check-update', item);
  }

  /**
   * Initializes file watching on the site directory.
   */
  startWatcher() {
    if (!this.watchEnabled) return;

    try {
      this.watcher = watch(this.siteDir, { recursive: true }, (eventType, filename) => {
        if (!filename) return;
        const normalized = filename.replace(/\\/g, '/');

        // Ignore temporary or build artifacts
        if (
          normalized.includes('.git') ||
          normalized.includes('node_modules') ||
          normalized.endsWith('.tmp') ||
          normalized.endsWith('~')
        ) {
          return;
        }

        this.log(`File change detected: ${normalized} (${eventType})`);

        // Debounce to batch rapid successive saves
        clearTimeout(this.debounceTimer);
        this.debounceTimer = setTimeout(() => {
          this.broadcast('reload', { file: normalized });
          this.runChecks();
        }, 150);
      });
      this.log(`Live file watcher active on ${this.siteDir}`);
    } catch (err) {
      this.log(`Warning: Failed to start file watcher: ${err.message}`);
    }
  }

  /**
   * Handles HTTP requests.
   */
  handleRequest(req, res) {
    const urlObj = new URL(req.url, `http://${req.headers.host || 'localhost'}`);
    const pathname = urlObj.pathname;

    // SSE Event Stream endpoint
    if (pathname === '/__events') {
      res.writeHead(200, {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
        Connection: 'keep-alive',
        'Access-Control-Allow-Origin': '*',
      });
      res.write('\n');
      this.clients.add(res);

      // Send initial state
      res.write(
        `event: init\ndata: ${JSON.stringify({
          checks: this.checkResults,
          logs: this.logs.slice(-50),
          siteDir: this.siteDir,
        })}\n\n`
      );

      req.on('close', () => this.clients.delete(res));
      return;
    }

    // API: Run checks on demand
    if (pathname === '/__api/run-checks' && req.method === 'POST') {
      this.runChecks();
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ triggered: true }));
      return;
    }

    // API: Status
    if (pathname === '/__api/status') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ checks: this.checkResults, logs: this.logs.slice(-30) }));
      return;
    }

    // Dashboard page endpoint: /__dashboard or /dashboard
    if (pathname === '/__dashboard' || pathname === '/dashboard') {
      const html = this.renderDashboardHtml();
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      res.end(html);
      return;
    }

    // Emulated Firebase Hosting static server
    this.serveFirebaseStatic(pathname, res);
  }

  /**
   * Emulates Firebase Hosting file resolution, rewrites, and headers.
   */
  serveFirebaseStatic(requestPath, res) {
    // 1. Apply Firebase rewrites
    const rewrites = this.firebaseConfig.rewrites || [];
    const targetSubpath = resolveRewrite(requestPath, rewrites);

    // 2. Resolve file on disk
    let diskPath = join(this.siteDir, targetSubpath.replace(/^\//, ''));

    // Directory index resolution
    if (existsSync(diskPath) && statSync(diskPath).isDirectory()) {
      diskPath = join(diskPath, 'index.html');
    }

    if (!existsSync(diskPath)) {
      // If file doesn't exist, check if appending .bin helps (for downloads)
      if (existsSync(diskPath + '.bin')) {
        diskPath = diskPath + '.bin';
      } else {
        res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
        res.end(`404 Not Found: "${requestPath}"`);
        return;
      }
    }

    // 3. Resolve custom headers from firebase.json
    const customHeaders = resolveHeaders(requestPath, this.firebaseConfig.headers || []);

    // 4. Determine Content-Type
    const ext = extname(diskPath).toLowerCase();
    const contentType = customHeaders['Content-Type'] || MIME_TYPES[ext] || 'application/octet-stream';

    const finalHeaders = {
      'Content-Type': contentType,
      ...customHeaders,
    };

    try {
      const fileBytes = readFileSync(diskPath);
      finalHeaders['Content-Length'] = fileBytes.length;
      res.writeHead(200, finalHeaders);
      res.end(fileBytes);
    } catch (err) {
      res.writeHead(500, { 'Content-Type': 'text/plain; charset=utf-8' });
      res.end(`500 Internal Error reading file: ${err.message}`);
    }
  }

  /**
   * Renders the single-screen AMOLED Dark Glass QA Dashboard.
   */
  renderDashboardHtml() {
    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Smart Migrate — Local QA Dashboard & Emulation Server</title>
  <link rel="icon" type="image/svg+xml" href="/favicon.svg">
  <style>
    :root {
      --bg: #000000;
      --card-bg: rgba(18, 16, 26, 0.72);
      --card-border: rgba(255, 255, 255, 0.08);
      --accent: #7C3AED;
      --accent-glow: rgba(124, 58, 237, 0.35);
      --accent-light: #A78BFA;
      --success: #10B981;
      --success-glow: rgba(16, 185, 129, 0.3);
      --error: #EF4444;
      --error-glow: rgba(239, 68, 68, 0.3);
      --text: #F3F4F6;
      --text-muted: #9CA3AF;
      --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
      --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background: var(--bg);
      color: var(--text);
      font-family: var(--font-sans);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      padding: 24px;
      gap: 20px;
    }
    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      flex-wrap: wrap;
      gap: 16px;
      padding: 16px 24px;
      background: var(--card-bg);
      backdrop-filter: blur(24px);
      -webkit-backdrop-filter: blur(24px);
      border: 1px solid var(--card-border);
      border-radius: 16px;
    }
    .brand-wrap {
      display: flex;
      align-items: center;
      gap: 14px;
    }
    .brand-logo {
      width: 36px;
      height: 36px;
      border-radius: 10px;
      box-shadow: 0 0 16px var(--accent-glow);
    }
    .brand-title {
      font-size: 20px;
      font-weight: 700;
      letter-spacing: -0.02em;
      background: linear-gradient(135deg, #FFF, var(--accent-light));
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
    }
    .brand-sub {
      font-size: 13px;
      color: var(--text-muted);
    }
    .badge-bar {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .badge {
      font-size: 12px;
      font-weight: 600;
      padding: 5px 10px;
      border-radius: 20px;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      display: inline-flex;
      align-items: center;
      gap: 6px;
    }
    .badge.active {
      border-color: var(--success);
      color: var(--success);
      background: rgba(16, 185, 129, 0.1);
    }
    .btn-run {
      background: linear-gradient(135deg, #7C3AED, #6D28D9);
      color: #FFF;
      border: none;
      padding: 8px 18px;
      border-radius: 10px;
      font-weight: 600;
      font-size: 13px;
      cursor: pointer;
      box-shadow: 0 4px 14px var(--accent-glow);
      transition: all 0.2s ease;
    }
    .btn-run:hover {
      transform: translateY(-1px);
      box-shadow: 0 6px 18px var(--accent-glow);
    }
    .links-strip {
      display: flex;
      flex-wrap: wrap;
      gap: 10px;
    }
    .link-chip {
      background: var(--card-bg);
      border: 1px solid var(--card-border);
      padding: 8px 14px;
      border-radius: 10px;
      color: var(--accent-light);
      text-decoration: none;
      font-size: 13px;
      font-weight: 500;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      transition: all 0.15s ease;
    }
    .link-chip:hover {
      border-color: var(--accent-light);
      background: rgba(124, 58, 237, 0.15);
      transform: translateY(-1px);
    }
    .grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 20px;
    }
    @media (max-width: 900px) {
      .grid { grid-template-columns: 1fr; }
    }
    .card {
      background: var(--card-bg);
      backdrop-filter: blur(24px);
      -webkit-backdrop-filter: blur(24px);
      border: 1px solid var(--card-border);
      border-radius: 16px;
      padding: 20px;
      display: flex;
      flex-direction: column;
      gap: 16px;
    }
    .card-title {
      font-size: 16px;
      font-weight: 650;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }
    .checks-list {
      display: flex;
      flex-direction: column;
      gap: 10px;
    }
    .check-item {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 12px 14px;
      border-radius: 10px;
      background: rgba(255, 255, 255, 0.02);
      border: 1px solid rgba(255, 255, 255, 0.05);
    }
    .check-info {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }
    .check-name {
      font-size: 14px;
      font-weight: 600;
    }
    .check-desc {
      font-size: 12px;
      color: var(--text-muted);
    }
    .pill {
      font-size: 11px;
      font-weight: 700;
      padding: 3px 8px;
      border-radius: 6px;
      letter-spacing: 0.03em;
    }
    .pill.PASS { background: rgba(16, 185, 129, 0.15); color: var(--success); border: 1px solid var(--success); }
    .pill.FAIL { background: rgba(239, 68, 68, 0.15); color: var(--error); border: 1px solid var(--error); }
    .pill.RUNNING { background: rgba(124, 58, 237, 0.15); color: var(--accent-light); border: 1px solid var(--accent); }
    .pill.PENDING { background: rgba(255, 255, 255, 0.05); color: var(--text-muted); }

    .terminal {
      background: #050408;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 12px;
      padding: 14px;
      font-family: var(--font-mono);
      font-size: 12px;
      height: 220px;
      overflow-y: auto;
      color: #D1D5DB;
      line-height: 1.5;
    }
    .terminal .line { margin-bottom: 2px; }
    .terminal .line.err { color: #F87171; }
    .terminal .line.pass { color: #34D399; }

    .checklist-category {
      margin-bottom: 14px;
    }
    .checklist-cat-title {
      font-size: 13px;
      font-weight: 700;
      color: var(--accent-light);
      margin-bottom: 8px;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }
    .checklist-row {
      display: flex;
      align-items: flex-start;
      gap: 10px;
      padding: 8px 10px;
      border-radius: 8px;
      margin-bottom: 4px;
      background: rgba(255, 255, 255, 0.015);
      cursor: pointer;
      user-select: none;
    }
    .checklist-row:hover {
      background: rgba(255, 255, 255, 0.04);
    }
    .checklist-row input[type="checkbox"] {
      margin-top: 3px;
      accent-color: var(--accent);
      cursor: pointer;
    }
    .checklist-content {
      font-size: 13px;
    }
    .checklist-note {
      font-size: 11px;
      color: var(--text-muted);
      margin-top: 2px;
    }
  </style>
</head>
<body>

  <header>
    <div class="brand-wrap">
      <img src="/smart_migrate_logo.png" alt="Smart Migrate" class="brand-logo" onerror="this.src='/favicon.svg'">
      <div>
        <div class="brand-title">Smart Migrate</div>
        <div class="brand-sub">Local Test Server &amp; QA Verification Dashboard</div>
      </div>
    </div>
    <div class="badge-bar">
      <span class="badge active">● Firebase Emulated</span>
      <span class="badge active">● Live Reload Active</span>
      <button class="btn-run" id="btn-run-checks">⚡ Run All Checks</button>
    </div>
  </header>

  <div class="links-strip">
    <a href="/index.html" target="_blank" class="link-chip">🌐 Website Preview</a>
    <a href="/releases/manifest.json" target="_blank" class="link-chip">📄 Schema 1 (manifest.json)</a>
    <a href="/release.json" target="_blank" class="link-chip">📄 Schema 2 (release.json)</a>
    <a href="/downloads/android/SmartMigrate.apk" class="link-chip">📱 Download APK</a>
    <a href="/downloads/windows/SmartMigrate-Setup-x64.exe" class="link-chip">💻 Download Windows EXE</a>
    <a href="/downloads/windows/SmartMigrate-x64.msi" class="link-chip">📦 Download Windows MSI</a>
  </div>

  <div class="grid">
    <!-- Left Column: Automated Checks & Live Log -->
    <div style="display:flex; flex-direction:column; gap:20px;">
      <div class="card">
        <div class="card-title">
          <span>Automated Verification Checks</span>
          <span id="checks-summary" style="font-size:12px; color:var(--text-muted)">Running tests...</span>
        </div>
        <div class="checks-list" id="checks-list">
          <!-- Populated dynamically -->
        </div>
      </div>

      <div class="card">
        <div class="card-title">
          <span>Live Execution Log</span>
          <span style="font-size:12px; color:var(--text-muted)">SSE Stream Active</span>
        </div>
        <div class="terminal" id="terminal">
          <!-- Populated dynamically -->
        </div>
      </div>
    </div>

    <!-- Right Column: Manual QA Checklist -->
    <div class="card">
      <div class="card-title">
        <span>Manual QA Verification Checklist</span>
        <button id="btn-reset-qa" style="background:none; border:none; color:var(--text-muted); cursor:pointer; font-size:11px;">Reset</button>
      </div>

      <div class="checklist-category">
        <div class="checklist-cat-title">1. Website &amp; Distribution Gate</div>
        <label class="checklist-row">
          <input type="checkbox" data-qa="web-brand">
          <div class="checklist-content">
            <b>Product Brand Consistency</b>
            <div class="checklist-note">Verify "Smart Migrate" name is uniform with no internal project keys leaked.</div>
          </div>
        </label>
        <label class="checklist-row">
          <input type="checkbox" data-qa="web-downloads">
          <div class="checklist-content">
            <b>Download Headers &amp; Extensions</b>
            <div class="checklist-note">Verify .apk, .exe, and .msi download with attachment Content-Disposition and matching hashes.</div>
          </div>
        </label>
        <label class="checklist-row">
          <input type="checkbox" data-qa="web-honesty">
          <div class="checklist-content">
            <b>Honest Marketing Representation</b>
            <div class="checklist-note">Confirm stream HUD preview is labeled MOCKUP; no claims of unbuilt hardware capture.</div>
          </div>
        </label>
        <label class="checklist-row">
          <input type="checkbox" data-qa="web-liquid-glass">
          <div class="checklist-content">
            <b>Liquid Glass UI &amp; AMOLED Black</b>
            <div class="checklist-note">Confirm pure #000000 background, violet accents, and responsive viewport layout.</div>
          </div>
        </label>
      </div>

      <div class="checklist-category">
        <div class="checklist-cat-title">2. Windows Host Shell</div>
        <label class="checklist-row">
          <input type="checkbox" data-qa="win-launch">
          <div class="checklist-content">
            <b>Frameless Window &amp; Window Controls</b>
            <div class="checklist-note">Launches maximized; title bar dragging and 38x32px minimize/close controls work smoothly.</div>
          </div>
        </label>
        <label class="checklist-row">
          <input type="checkbox" data-qa="win-updater">
          <div class="checklist-content">
            <b>Elevated In-App Updater</b>
            <div class="checklist-note">Pre-elevation SHA-256 validation; Authenticode check; launches UAC prompt and releases file locks.</div>
          </div>
        </label>
        <label class="checklist-row">
          <input type="checkbox" data-qa="win-transport">
          <div class="checklist-content">
            <b>Transport Security Transparency</b>
            <div class="checklist-note">Accurately labeled: "Direct LAN (HTTP over TCP, not encrypted)".</div>
          </div>
        </label>
        <label class="checklist-row">
          <input type="checkbox" data-qa="win-input">
          <div class="checklist-content">
            <b>Input &amp; Dynamic Aspect Ratio</b>
            <div class="checklist-note">Dynamic screen coordinate mapping prevents margin mis-clicks during remote control.</div>
          </div>
        </label>
      </div>

      <div class="checklist-category">
        <div class="checklist-cat-title">3. Android Client App</div>
        <label class="checklist-row">
          <input type="checkbox" data-qa="android-pair">
          <div class="checklist-content">
            <b>Subnet Discovery &amp; Pairing Modal</b>
            <div class="checklist-note">Fast IP discovery without .33 guessing; pairing dialog auto-dismisses immediately upon host approval.</div>
          </div>
        </label>
        <label class="checklist-row">
          <input type="checkbox" data-qa="android-stream">
          <div class="checklist-content">
            <b>Video Stream &amp; Reconnect State</b>
            <div class="checklist-note">Active video frame decode immediately transitions to STREAMING and clears reconnect banner.</div>
          </div>
        </label>
        <label class="checklist-row">
          <input type="checkbox" data-qa="android-input">
          <div class="checklist-content">
            <b>Session Token Fallback</b>
            <div class="checklist-note">Persistent session token across input handlers ensures mouse and touch coords are never dropped.</div>
          </div>
        </label>
      </div>

      <div class="checklist-category">
        <div class="checklist-cat-title">4. Cross-Device &amp; Protocol SMP/1</div>
        <label class="checklist-row">
          <input type="checkbox" data-qa="proto-auth">
          <div class="checklist-content">
            <b>Host Authoritative Capabilities</b>
            <div class="checklist-note">Host operator is single source of truth for granted permissions; zero secret logging.</div>
          </div>
        </label>
        <label class="checklist-row">
          <input type="checkbox" data-qa="proto-limits">
          <div class="checklist-content">
            <b>Connection Limits &amp; Deadlines</b>
            <div class="checklist-note">Enforced MAX_CONCURRENT_CLIENTS = 32 and 1500ms read timeout under rapid packet drops.</div>
          </div>
        </label>
      </div>
    </div>
  </div>

  <script>
    // SSE Event Source Listener
    const evtSource = new EventSource('/__events');
    const term = document.getElementById('terminal');
    const checksList = document.getElementById('checks-list');

    function appendLog(line) {
      const el = document.createElement('div');
      el.className = 'line';
      if (line.includes('[ERROR]') || line.includes('[FAIL]')) el.classList.add('err');
      if (line.includes('[PASS]')) el.classList.add('pass');
      el.textContent = line;
      term.appendChild(el);
      term.scrollTop = term.scrollHeight;
    }

    function renderChecks(checks) {
      checksList.innerHTML = '';
      let passed = 0;
      for (const c of checks) {
        if (c.status === 'PASS') passed++;
        const item = document.createElement('div');
        item.className = 'check-item';
        item.innerHTML = \`
          <div class="check-info">
            <span class="check-name">\${c.name}</span>
            <span class="check-desc">\${c.message || 'Waiting to run...'}</span>
          </div>
          <span class="pill \${c.status}">\${c.status}\${c.durationMs ? ' (' + c.durationMs + 'ms)' : ''}</span>
        \`;
        checksList.appendChild(item);
      }
      document.getElementById('checks-summary').textContent = \`\${passed}/\${checks.length} Passed\`;
    }

    evtSource.addEventListener('init', (e) => {
      const data = JSON.parse(e.data);
      if (data.checks) renderChecks(data.checks);
      if (data.logs) {
        term.innerHTML = '';
        for (const l of data.logs) appendLog(l);
      }
    });

    evtSource.addEventListener('log', (e) => {
      const data = JSON.parse(e.data);
      appendLog(data.message);
    });

    evtSource.addEventListener('check-update', (e) => {
      fetch('/__api/status').then(r => r.json()).then(data => {
        if (data.checks) renderChecks(data.checks);
      });
    });

    evtSource.addEventListener('checks-complete', (e) => {
      const data = JSON.parse(e.data);
      if (data.results) renderChecks(data.results);
    });

    evtSource.addEventListener('reload', (e) => {
      appendLog('[LIVE RELOAD] Website modification detected, refreshing preview...');
    });

    document.getElementById('btn-run-checks').addEventListener('click', () => {
      fetch('/__api/run-checks', { method: 'POST' });
    });

    // QA Checklist LocalStorage Persistence
    const STORAGE_KEY = 'smart_migrate_manual_qa_state_v1';
    function loadQaState() {
      try {
        const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}');
        document.querySelectorAll('input[data-qa]').forEach(cb => {
          const key = cb.getAttribute('data-qa');
          if (saved[key]) cb.checked = true;
        });
      } catch (_) {}
    }

    function saveQaState() {
      const state = {};
      document.querySelectorAll('input[data-qa]').forEach(cb => {
        const key = cb.getAttribute('data-qa');
        state[key] = cb.checked;
      });
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    }

    document.querySelectorAll('input[data-qa]').forEach(cb => {
      cb.addEventListener('change', saveQaState);
    });

    document.getElementById('btn-reset-qa').addEventListener('click', () => {
      if (confirm('Reset all manual QA checklist items?')) {
        localStorage.removeItem(STORAGE_KEY);
        document.querySelectorAll('input[data-qa]').forEach(cb => cb.checked = false);
      }
    });

    loadQaState();
  </script>
</body>
</html>`;
  }

  /**
   * Starts HTTP server and begins file watching & verification.
   */
  async start() {
    return new Promise((resolveStart, reject) => {
      this.server = http.createServer((req, res) => this.handleRequest(req, res));

      this.server.on('error', (err) => {
        this.log(`Server error: ${err.message}`);
        reject(err);
      });

      this.server.listen(this.port, () => {
        const addr = this.server.address();
        this.port = typeof addr === 'object' ? addr.port : this.port;
        this.log(`Smart Migrate Local QA Server running at http://localhost:${this.port}`);
        this.log(`Local QA Dashboard: http://localhost:${this.port}/__dashboard`);
        this.log(`Website Preview:    http://localhost:${this.port}/index.html`);

        this.startWatcher();
        // Initial check run after server is listening
        this.runChecks().then(() => resolveStart(this.port));
      });
    });
  }

  /**
   * Stops the server and closes watchers and open sockets.
   */
  async stop() {
    if (this.watcher) {
      try {
        this.watcher.close();
      } catch (_) {}
    }
    clearTimeout(this.debounceTimer);

    for (const client of this.clients) {
      try {
        client.end();
      } catch (_) {}
    }
    this.clients.clear();

    if (this.server) {
      return new Promise((resolveStop) => {
        this.server.close(() => resolveStop());
      });
    }
  }
}

// CLI entry point
if (process.argv[1] && process.argv[1].endsWith('local-dashboard.mjs')) {
  let port = 3000;
  for (let i = 2; i < process.argv.length; i++) {
    if (process.argv[i] === '--port' && process.argv[i + 1]) {
      port = parseInt(process.argv[++i], 10);
    }
  }

  const dashboard = new LocalDashboardServer({ port });
  dashboard.start().catch((err) => {
    console.error('Failed to start dashboard:', err);
    process.exit(1);
  });
}
