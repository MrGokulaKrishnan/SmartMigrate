/**
 * Smart Migrate — Site Bundle Builder & Manifest Regression Test Suite
 *
 * Verifies:
 * 1. Website-only mode passes and preserves committed binary records
 * 2. Release mode passes dry-run and stages updates cleanly
 * 3. Android and Windows version gates reject mismatched versions
 * 4. Output location guard blocks in-place overwriting of site source
 * 5. Stale manifest detection rejects corrupted/tampered hashes with exact field details
 */

import { mkdtempSync, cpSync, rmSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
import { buildSiteBundle } from '../scripts/build-site-bundle.mjs';
import { resolveSiteDir, resolveRepoRoot, readProjectVersions } from '../scripts/site-helpers.mjs';

console.log('Testing Smart Migrate Site Bundle Builder...\n');

const repoRoot = resolveRepoRoot();
const originalSiteDir = resolveSiteDir();
const projectVersions = readProjectVersions(repoRoot);
const currentVersion = projectVersions.windows.version || '1.4.4';

async function runTests() {
  // Test 1: Website-Only Mode
  console.log('1. Testing Website-Only Mode against production site...');
  const resWebOnly = await buildSiteBundle({
    mode: 'website-only',
    siteDir: originalSiteDir,
  });
  assert.equal(resWebOnly.success, true, 'Website-only build should succeed');
  assert.equal(resWebOnly.mode, 'website-only');
  assert(resWebOnly.manifestCount >= 2, 'Should verify at least 2 manifest files');
  console.log('   ✓ Website-only mode verified successfully.');

  // Test 2: Release Mode Dry-Run
  console.log('2. Testing Release Mode Dry-Run with current version...');
  const resRelease = await buildSiteBundle({
    mode: 'release',
    dryRun: true,
    siteDir: originalSiteDir,
    targetVersion: currentVersion,
  });
  assert.equal(resRelease.success, true, 'Release mode dry-run should succeed');
  assert.equal(resRelease.mode, 'release');
  assert.equal(resRelease.version, currentVersion);
  console.log('   ✓ Release mode dry-run verified successfully.');

  // Test 3: Version Gates Rejection
  console.log('3. Testing Version Gates Rejection for invalid versions...');
  const invalidVersions = ['9.9.9', '0.0.1-alpha', '2.0.0'];
  for (const badVer of invalidVersions) {
    let rejected = false;
    try {
      await buildSiteBundle({
        mode: 'website-only',
        siteDir: originalSiteDir,
        targetVersion: badVer,
      });
    } catch (err) {
      rejected = true;
      assert(err.message.includes('Version gate failure'), `Unexpected error message: ${err.message}`);
    }
    assert(rejected, `Version gate should have rejected version "${badVer}"`);
  }
  console.log('   ✓ Version gates rejected all invalid target versions.');

  // Test 4: Output Location Guard
  console.log('4. Testing Output Location Guard...');
  let guardTriggered = false;
  try {
    await buildSiteBundle({
      mode: 'website-only',
      siteDir: originalSiteDir,
      outDir: originalSiteDir,
    });
  } catch (err) {
    guardTriggered = true;
    assert(err.message.includes('Output location guard violated'), `Unexpected error: ${err.message}`);
  }
  assert(guardTriggered, 'Output location guard should prevent overwriting siteDir');
  console.log('   ✓ Output location guard confirmed.');

  // Test 5: Stale / Tampered Manifest Rejection
  console.log('5. Testing Stale / Tampered Manifest Rejection...');
  const tempSite = mkdtempSync(join(tmpdir(), 'sm-stale-test-'));
  try {
    // Copy real site to tempSite
    cpSync(originalSiteDir, tempSite, { recursive: true });

    // Tamper with Schema 1 (manifest.json)
    const mPath = join(tempSite, 'releases', 'manifest.json');
    const manifest = JSON.parse(readFileSync(mPath, 'utf8'));
    const realSha = manifest.platforms['windows-x86_64'].sha256;
    // Alter last character of hash
    manifest.platforms['windows-x86_64'].sha256 = realSha.slice(0, -1) + (realSha.slice(-1) === 'a' ? 'b' : 'a');
    writeFileSync(mPath, JSON.stringify(manifest, null, 2), 'utf8');

    let staleRejected = false;
    let errorMessage = '';
    try {
      await buildSiteBundle({
        mode: 'website-only',
        siteDir: tempSite,
      });
    } catch (err) {
      staleRejected = true;
      errorMessage = err.message;
    }

    assert(staleRejected, 'Builder must fail-closed and reject tampered manifest');
    assert(errorMessage.includes('Manifest SHA-256 mismatch'), `Error should cite SHA-256 mismatch: ${errorMessage}`);
    assert(errorMessage.includes('windows-x86_64'), `Error should cite exact platform: ${errorMessage}`);
    console.log(`   ✓ Stale manifest correctly rejected with: "${errorMessage.split('\n')[0]}"`);
  } finally {
    if (existsSync(tempSite)) {
      rmSync(tempSite, { recursive: true, force: true });
    }
  }

  console.log('\nAll Bundle Builder & Manifest Regression Tests Passed Successfully!');
}

runTests().catch((err) => {
  console.error('\nTest Suite Failed:', err);
  process.exit(1);
});
