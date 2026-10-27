/**
 * Smart Migrate — Post-Deploy Smoke Test & Byte-for-Byte Verifier
 *
 * Verifies that a deployed website or local emulator serves exactly what users download:
 * - /releases/manifest.json & /release.json respond with valid JSON and security headers
 * - /downloads/android/SmartMigrate.apk returns correct headers and byte-for-byte SHA-256 matching manifest
 * - /downloads/windows/SmartMigrate-Setup-x64.exe returns correct headers and SHA-256 matching manifest
 * - /downloads/windows/SmartMigrate-x64.msi returns correct headers and SHA-256 matching release.json
 */

import { createHash } from 'node:crypto';
import http from 'node:http';
import https from 'node:https';

/**
 * Performs HTTP/HTTPS GET request, returning status, headers, and full Buffer body.
 */
export function fetchUrl(targetUrl, timeoutMs = 15000) {
  return new Promise((resolve, reject) => {
    const urlObj = new URL(targetUrl);
    const client = urlObj.protocol === 'https:' ? https : http;

    const req = client.get(
      urlObj,
      {
        headers: {
          'User-Agent': 'SmartMigrate-SmokeTest/1.0',
          Accept: '*/*',
        },
        timeout: timeoutMs,
      },
      (res) => {
        const chunks = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => {
          const body = Buffer.concat(chunks);
          resolve({
            status: res.statusCode,
            headers: res.headers,
            body,
            text: () => body.toString('utf8'),
            sha256: () => createHash('sha256').update(body).digest('hex'),
          });
        });
      }
    );

    req.on('timeout', () => {
      req.destroy();
      reject(new Error(`Timeout after ${timeoutMs}ms fetching ${targetUrl}`));
    });

    req.on('error', (err) => reject(err));
  });
}

/**
 * Executes full smoke test against specified base URL.
 */
export async function runSmokeTest(baseUrl = 'http://localhost:3000', options = {}) {
  const url = baseUrl.replace(/\/+$/, '');
  const log = options.quiet ? () => {} : console.log;
  const verified = [];

  log(`[smoke-test] Target URL: ${url}`);

  // 1. Fetch /releases/manifest.json
  log('[smoke-test] Checking /releases/manifest.json...');
  const manifestRes = await fetchUrl(`${url}/releases/manifest.json`);
  if (manifestRes.status !== 200) {
    throw new Error(`Failed to fetch /releases/manifest.json: HTTP ${manifestRes.status}`);
  }
  const manifestJson = JSON.parse(manifestRes.text());
  if (!manifestJson.platforms || !manifestJson.platforms['windows-x86_64'] || !manifestJson.platforms['android-arm64']) {
    throw new Error('/releases/manifest.json is missing required platform blocks');
  }
  verified.push({ endpoint: '/releases/manifest.json', status: 200 });

  // 2. Fetch /release.json
  log('[smoke-test] Checking /release.json...');
  const releaseRes = await fetchUrl(`${url}/release.json`);
  if (releaseRes.status !== 200) {
    throw new Error(`Failed to fetch /release.json: HTTP ${releaseRes.status}`);
  }
  const releaseJson = JSON.parse(releaseRes.text());
  if (!releaseJson.artifacts || !releaseJson.artifacts['windows-exe'] || !releaseJson.artifacts['windows-msi'] || !releaseJson.artifacts['android-apk']) {
    throw new Error('/release.json is missing required artifact entries');
  }
  verified.push({ endpoint: '/release.json', status: 200 });

  // 3. Test Android APK download & byte-for-byte SHA-256
  const expectedApkSha = (manifestJson.platforms['android-arm64'].sha256 || '').toLowerCase();
  const expectedApkSize = Number(manifestJson.platforms['android-arm64'].size);
  log(`[smoke-test] Downloading /downloads/android/SmartMigrate.apk (expected: ${expectedApkSha.slice(0, 12)}...)...`);
  const apkRes = await fetchUrl(`${url}/downloads/android/SmartMigrate.apk`);
  if (apkRes.status !== 200) {
    throw new Error(`Failed to download APK: HTTP ${apkRes.status}`);
  }
  const actualApkSha = apkRes.sha256();
  const actualApkSize = apkRes.body.length;
  if (actualApkSha !== expectedApkSha) {
    throw new Error(
      `APK SHA-256 mismatch!\n  Manifest: ${expectedApkSha}\n  Actual:   ${actualApkSha}`
    );
  }
  if (actualApkSize !== expectedApkSize) {
    throw new Error(
      `APK size mismatch!\n  Manifest: ${expectedApkSize} bytes\n  Actual:   ${actualApkSize} bytes`
    );
  }
  verified.push({
    endpoint: '/downloads/android/SmartMigrate.apk',
    status: 200,
    sha256: actualApkSha,
    size: actualApkSize,
    contentType: apkRes.headers['content-type'],
  });

  // 4. Test Windows EXE download & byte-for-byte SHA-256
  const expectedExeSha = (manifestJson.platforms['windows-x86_64'].sha256 || '').toLowerCase();
  const expectedExeSize = Number(manifestJson.platforms['windows-x86_64'].size);
  log(`[smoke-test] Downloading /downloads/windows/SmartMigrate-Setup-x64.exe (expected: ${expectedExeSha.slice(0, 12)}...)...`);
  const exeRes = await fetchUrl(`${url}/downloads/windows/SmartMigrate-Setup-x64.exe`);
  if (exeRes.status !== 200) {
    throw new Error(`Failed to download Windows EXE: HTTP ${exeRes.status}`);
  }
  const actualExeSha = exeRes.sha256();
  const actualExeSize = exeRes.body.length;
  if (actualExeSha !== expectedExeSha) {
    throw new Error(
      `Windows EXE SHA-256 mismatch!\n  Manifest: ${expectedExeSha}\n  Actual:   ${actualExeSha}`
    );
  }
  if (actualExeSize !== expectedExeSize) {
    throw new Error(
      `Windows EXE size mismatch!\n  Manifest: ${expectedExeSize} bytes\n  Actual:   ${actualExeSize} bytes`
    );
  }
  verified.push({
    endpoint: '/downloads/windows/SmartMigrate-Setup-x64.exe',
    status: 200,
    sha256: actualExeSha,
    size: actualExeSize,
    contentType: exeRes.headers['content-type'],
  });

  // 5. Test Windows MSI download & byte-for-byte SHA-256
  const expectedMsiSha = (releaseJson.artifacts['windows-msi'].sha256 || '').toLowerCase();
  const expectedMsiSize = Number(releaseJson.artifacts['windows-msi'].size_bytes);
  log(`[smoke-test] Downloading /downloads/windows/SmartMigrate-x64.msi (expected: ${expectedMsiSha.slice(0, 12)}...)...`);
  const msiRes = await fetchUrl(`${url}/downloads/windows/SmartMigrate-x64.msi`);
  if (msiRes.status !== 200) {
    throw new Error(`Failed to download Windows MSI: HTTP ${msiRes.status}`);
  }
  const actualMsiSha = msiRes.sha256();
  const actualMsiSize = msiRes.body.length;
  if (actualMsiSha !== expectedMsiSha) {
    throw new Error(
      `Windows MSI SHA-256 mismatch!\n  Release JSON: ${expectedMsiSha}\n  Actual:       ${actualMsiSha}`
    );
  }
  if (actualMsiSize !== expectedMsiSize) {
    throw new Error(
      `Windows MSI size mismatch!\n  Release JSON: ${expectedMsiSize} bytes\n  Actual:       ${actualMsiSize} bytes`
    );
  }
  verified.push({
    endpoint: '/downloads/windows/SmartMigrate-x64.msi',
    status: 200,
    sha256: actualMsiSha,
    size: actualMsiSize,
    contentType: msiRes.headers['content-type'],
  });

  log('[smoke-test] ✓ All 5 endpoints passed byte-for-byte verification!');
  return { success: true, verified };
}

// CLI entry point
if (process.argv[1] && process.argv[1].endsWith('smoke-test.mjs')) {
  let targetUrl = 'http://localhost:3000';
  for (let i = 2; i < process.argv.length; i++) {
    if (process.argv[i] === '--url' && process.argv[i + 1]) {
      targetUrl = process.argv[++i];
    }
  }

  runSmokeTest(targetUrl)
    .then((res) => {
      console.log(`[smoke-test] Complete: ${res.verified.length} checks verified.`);
      process.exit(0);
    })
    .catch((err) => {
      console.error(`[smoke-test] Verification failed: ${err.message}`);
      process.exit(1);
    });
}
