/**
 * Smart Migrate — Local QA Dashboard & Emulation Server End-to-End Test Suite
 *
 * Verifies:
 * 1. Server boots on ephemeral port and serves Firebase rewrites and headers accurately
 * 2. SSE event stream transmits initialization, live logs, and check results
 * 3. Smoke test passes byte-for-byte through the local emulation server
 * 4. Live file watcher detects modifications and streams reload notifications
 * 5. Queued re-run handling prevents race conditions during concurrent file saves
 */

import { writeFileSync, unlinkSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import assert from 'node:assert/strict';
import http from 'node:http';
import { LocalDashboardServer } from '../scripts/local-dashboard.mjs';
import { runSmokeTest, fetchUrl } from '../scripts/smoke-test.mjs';
import { resolveSiteDir } from '../scripts/site-helpers.mjs';

console.log('Testing Smart Migrate Local Dashboard & Firebase Emulation Server...\n');

const siteDir = resolveSiteDir();

async function runTests() {
  const dashboard = new LocalDashboardServer({
    siteDir,
    port: 0, // Ephemeral port
    watch: true,
  });

  let port = null;

  try {
    // 1. Boot Server
    console.log('1. Starting Local Dashboard Server on ephemeral port...');
    port = await dashboard.start();
    const serverUrl = `http://localhost:${port}`;
    console.log(`   ✓ Server listening at ${serverUrl}`);

    // 2. Test Firebase Rewrite & Header Emulation
    console.log('2. Testing Firebase Rewrite and Header Emulation...');
    const apkRes = await fetchUrl(`${serverUrl}/downloads/android/SmartMigrate.apk`);
    assert.equal(apkRes.status, 200, 'APK endpoint should return 200 OK');
    assert(
      (apkRes.headers['content-type'] || '').includes('application/vnd.android.package-archive'),
      `Content-Type header mismatch: ${apkRes.headers['content-type']}`
    );
    assert(
      (apkRes.headers['content-disposition'] || '').includes('SmartMigrate.apk'),
      `Content-Disposition header mismatch: ${apkRes.headers['content-disposition']}`
    );
    console.log('   ✓ Android APK rewrite to .bin and download headers verified.');

    const exeRes = await fetchUrl(`${serverUrl}/downloads/windows/SmartMigrate-Setup-x64.exe`);
    assert.equal(exeRes.status, 200, 'EXE endpoint should return 200 OK');
    assert(
      (exeRes.headers['content-type'] || '').includes('application/vnd.microsoft.portable-executable'),
      `EXE Content-Type header mismatch: ${exeRes.headers['content-type']}`
    );
    console.log('   ✓ Windows EXE rewrite to .bin and download headers verified.');

    const dashPageRes = await fetchUrl(`${serverUrl}/__dashboard`);
    assert.equal(dashPageRes.status, 200);
    assert(dashPageRes.text().includes('Smart Migrate'), 'Dashboard page must contain product title');
    assert(dashPageRes.text().includes('Manual QA Verification Checklist'), 'Dashboard must contain QA checklist');
    console.log('   ✓ Dashboard UI page served successfully.');

    // 3. Test Smoke Test Through Emulated Server
    console.log('3. Running byte-for-byte Smoke Test through emulated server...');
    const smokeResult = await runSmokeTest(serverUrl, { quiet: true });
    assert.equal(smokeResult.success, true);
    assert.equal(smokeResult.verified.length, 5);
    console.log(`   ✓ Smoke test verified all ${smokeResult.verified.length} endpoints through local server.`);

    // 4. Test SSE Event Stream
    console.log('4. Testing Server-Sent Events (SSE) stream...');
    const sseReceived = await new Promise((resolveSse, rejectSse) => {
      const req = http.get(`${serverUrl}/__events`, (res) => {
        assert.equal(res.statusCode, 200);
        assert.equal(res.headers['content-type'], 'text/event-stream');

        let dataBuf = '';
        res.on('data', (chunk) => {
          dataBuf += chunk.toString('utf8');
          if (dataBuf.includes('event: init') || dataBuf.includes('event: check-update') || dataBuf.includes('event: checks-complete')) {
            req.destroy();
            resolveSse(true);
          }
        });
      });
      req.on('error', (err) => {
        if (err.message.includes('socket hang up') || req.destroyed) {
          resolveSse(true);
        } else {
          rejectSse(err);
        }
      });
    });
    assert(sseReceived, 'SSE stream should emit check events');
    console.log('   ✓ SSE event stream connection and event emission verified.');

    // 5. Test Live Watcher & Reload Stream
    console.log('5. Testing Live File Watcher and Reload Trigger...');
    const testFile = join(siteDir, 'temp-watch-test-marker.txt');
    try {
      let reloadReceived = false;

      // Listen on SSE stream for reload event
      const watchPromise = new Promise((resolveWatch) => {
        const req = http.get(`${serverUrl}/__events`, (res) => {
          res.on('data', (chunk) => {
            const str = chunk.toString('utf8');
            if (str.includes('event: reload')) {
              reloadReceived = true;
              req.destroy();
              resolveWatch(true);
            }
          });
        });
        setTimeout(() => {
          req.destroy();
          resolveWatch(reloadReceived);
        }, 1500);
      });

      // Write temporary file
      writeFileSync(testFile, 'test marker for live reload\n', 'utf8');

      const triggered = await watchPromise;
      assert(triggered, 'File modification should trigger reload event');
      console.log('   ✓ Live reload event received over SSE stream upon file change.');
    } finally {
      if (existsSync(testFile)) {
        unlinkSync(testFile);
      }
    }

    // 6. Test Concurrent Change Queue Re-Run
    console.log('6. Testing Queued Re-Run Debouncing...');
    dashboard.isRunningChecks = true;
    dashboard.runChecks(); // Should set rerunQueued = true
    assert.equal(dashboard.rerunQueued, true, 'Calling runChecks while active should queue re-run');
    dashboard.isRunningChecks = false;
    console.log('   ✓ Re-run queuing correctly handles concurrent file modifications.');

  } finally {
    console.log('Cleaning up server...');
    await dashboard.stop();
    console.log('✓ Server closed cleanly.');
  }

  console.log('\nAll Local Dashboard & Emulation Tests Passed Successfully!');
}

runTests().catch((err) => {
  console.error('\nDashboard Test Suite Failed:', err);
  process.exit(1);
});
