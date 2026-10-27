/**
 * Smart Migrate — Automated Website & App Deployment Pipeline
 *
 * Automates:
 * 1. Verification of all manifests and download binaries
 * 2. Brand and release notes integrity tests
 * 3. Assembly of release bundle (excluding ignored files)
 * 4. Firebase Hosting deployment (firebase deploy --only hosting --non-interactive)
 * 5. Post-deployment smoke test verification against live production URLs
 */

import { spawnSync } from 'node:child_process';
import { resolve, join } from 'node:path';
import { resolveSiteDir, resolveRepoRoot, verifyAllSiteManifests, updateReleaseTimestamps } from './site-helpers.mjs';
import { buildSiteBundle } from './build-site-bundle.mjs';
import { runSmokeTest } from './smoke-test.mjs';

async function main() {
  const args = process.argv.slice(2);
  const dryRun = args.includes('--dry-run');
  const isRelease = args.includes('--release');
  const repoRoot = resolveRepoRoot();
  const siteDir = resolveSiteDir();

  console.log('=== Smart Migrate Website Deployment Pipeline ===\n');

  // 0. Update release timestamps to current deploy time
  if (!dryRun) {
    const timeRes = updateReleaseTimestamps(siteDir);
    console.log(`0. Synchronized release timestamps to current deploy time (${timeRes.timestamp}).`);
  }

  // 1. Run Pre-Deploy Verification
  console.log('1. Running Pre-Deploy Integrity Gates...');
  const brandCheck = spawnSync('node', ['scripts/check-brand.mjs'], { cwd: repoRoot, stdio: 'inherit' });
  if (brandCheck.status !== 0) {
    throw new Error('Brand naming check failed');
  }

  const notesCheck = spawnSync('node', ['tests/release_notes_test.mjs'], { cwd: repoRoot, stdio: 'inherit' });
  if (notesCheck.status !== 0) {
    throw new Error('Release notes test failed');
  }

  const manifestResult = verifyAllSiteManifests(siteDir);
  console.log(`   ✓ Verified ${manifestResult.verified.length} manifests and binary checksums.`);

  // 2. Build Site Bundle
  console.log('\n2. Assembling Site Bundle...');
  const bundleResult = await buildSiteBundle({
    mode: isRelease ? 'release' : 'website-only',
    siteDir,
    dryRun,
  });
  console.log(`   ✓ Bundle assembled successfully (${bundleResult.mode} mode).`);

  if (dryRun) {
    console.log('\n[DRY RUN] Skipping live Firebase deploy.');
    return;
  }

  // 3. Deploy to Firebase Hosting
  console.log('\n3. Deploying to Firebase Hosting...');
  const npxCmd = process.platform === 'win32' ? 'npx.cmd' : 'npx';
  const deploy = spawnSync(
    npxCmd,
    ['--yes', 'firebase-tools', 'deploy', '--only', 'hosting', '--non-interactive'],
    {
      cwd: siteDir,
      stdio: 'inherit',
      shell: true,
    }
  );

  if (deploy.status !== 0) {
    throw new Error(`Firebase deploy failed with code ${deploy.status}`);
  }
  console.log('   ✓ Firebase Hosting deployment completed successfully.');

  // 4. Post-Deploy Live Smoke Test
  console.log('\n4. Executing Post-Deploy Smoke Test on Production...');
  try {
    const smokeRes = await runSmokeTest('https://smartmigrate.web.app');
    console.log(`   ✓ Live smoke test verified all ${smokeRes.verified.length} endpoints!`);
  } catch (err) {
    console.warn(`   Warning: Production smoke test check encountered: ${err.message}`);
  }

  console.log('\n=== Deployment Pipeline Completed Successfully ===');
}

main().catch((err) => {
  console.error('\nDeployment Pipeline Failed:', err.message);
  process.exit(1);
});
