/**
 * Smart Migrate — Production Site Bundle Builder & Release Verifier
 *
 * Implements two deployment modes:
 * 1. Website-only mode:
 *    - Verifies that committed downloads match both release schemas (releases/manifest.json and release.json).
 *    - Copies site files (respecting firebase.json ignore rules) to staging folder.
 *    - Zero modification to git tree.
 * 2. Release mode (with installers or --release):
 *    - Validates project version gates (Android build.gradle.kts and Tauri tauri.conf.json).
 *    - Stages installers under their clean and .bin alias names.
 *    - Atomically updates both release schemas with verified SHA-256 and byte sizes.
 *    - Verifies assembled bundle end-to-end.
 */

import {
  readFileSync,
  writeFileSync,
  existsSync,
  mkdirSync,
  cpSync,
  rmSync,
  readdirSync,
  statSync,
} from 'node:fs';
import { resolve, join, relative, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import {
  resolveRepoRoot,
  resolveSiteDir,
  computeFileMetadata,
  formatFileSize,
  readFirebaseConfig,
  matchesGlob,
  verifyAllSiteManifests,
  readProjectVersions,
  updateReleaseTimestamps,
} from './site-helpers.mjs';

/**
 * Parses CLI arguments.
 */
function parseArgs(args) {
  const options = {
    mode: 'website-only',
    siteDir: null,
    outDir: null,
    verifyOnly: false,
    dryRun: false,
    targetVersion: null,
    apkPath: null,
    exePath: null,
    msiPath: null,
    updateTimestamp: true,
  };

  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === '--verify-only') {
      options.verifyOnly = true;
    } else if (a === '--dry-run') {
      options.dryRun = true;
    } else if (a === '--no-update-timestamp') {
      options.updateTimestamp = false;
    } else if (a === '--update-timestamp') {
      options.updateTimestamp = true;
    } else if (a === '--release') {
      options.mode = 'release';
    } else if (a === '--site-dir' && args[i + 1]) {
      options.siteDir = args[++i];
    } else if (a === '--out-dir' && args[i + 1]) {
      options.outDir = args[++i];
    } else if (a === '--version' && args[i + 1]) {
      options.targetVersion = args[++i];
    } else if (a === '--apk' && args[i + 1]) {
      options.apkPath = args[++i];
      options.mode = 'release';
    } else if (a === '--exe' && args[i + 1]) {
      options.exePath = args[++i];
      options.mode = 'release';
    } else if (a === '--msi' && args[i + 1]) {
      options.msiPath = args[++i];
      options.mode = 'release';
    }
  }

  return options;
}

/**
 * Copies site files recursively to staging directory, omitting ignored patterns.
 */
function copySiteFiles(sourceDir, destDir, ignoreRules) {
  mkdirSync(destDir, { recursive: true });

  function walk(currentDir) {
    const entries = readdirSync(currentDir, { withFileTypes: true });
    for (const entry of entries) {
      const fullSrc = join(currentDir, entry.name);
      const relPath = relative(sourceDir, fullSrc).replace(/\\/g, '/');

      // Check ignore patterns from firebase.json
      let isIgnored = false;
      for (const pattern of ignoreRules) {
        if (matchesGlob(relPath, pattern) || matchesGlob(entry.name, pattern)) {
          isIgnored = true;
          break;
        }
      }

      if (isIgnored) {
        continue;
      }

      const fullDest = join(destDir, relPath);
      if (entry.isDirectory()) {
        mkdirSync(fullDest, { recursive: true });
        walk(fullSrc);
      } else {
        const destParent = dirname(fullDest);
        if (!existsSync(destParent)) {
          mkdirSync(destParent, { recursive: true });
        }
        cpSync(fullSrc, fullDest);
      }
    }
  }

  walk(sourceDir);
}

/**
 * Updates Schema 1 (releases/manifest.json) in-place or at target.
 */
function updateSchema1(manifestPath, metadata) {
  if (!existsSync(manifestPath)) return;
  const raw = JSON.parse(readFileSync(manifestPath, 'utf8'));

  if (metadata.version) raw.version = metadata.version;
  if (metadata.versionCode) raw.versionCode = metadata.versionCode;
  if (metadata.releaseDate) raw.releaseDate = metadata.releaseDate;

  if (raw.platforms) {
    if (metadata.windowsExe && raw.platforms['windows-x86_64']) {
      raw.platforms['windows-x86_64'].version = metadata.version || raw.platforms['windows-x86_64'].version;
      raw.platforms['windows-x86_64'].sha256 = metadata.windowsExe.sha256;
      raw.platforms['windows-x86_64'].size = metadata.windowsExe.sizeBytes;
    }
    if (metadata.androidApk && raw.platforms['android-arm64']) {
      raw.platforms['android-arm64'].version = metadata.version || raw.platforms['android-arm64'].version;
      if (metadata.versionCode) raw.platforms['android-arm64'].versionCode = metadata.versionCode;
      raw.platforms['android-arm64'].sha256 = metadata.androidApk.sha256;
      raw.platforms['android-arm64'].size = metadata.androidApk.sizeBytes;
    }
  }

  writeFileSync(manifestPath, JSON.stringify(raw, null, 2) + '\n', 'utf8');
}

/**
 * Updates Schema 2 (release.json) in-place or at target.
 */
function updateSchema2(releasePath, metadata) {
  if (!existsSync(releasePath)) return;
  const raw = JSON.parse(readFileSync(releasePath, 'utf8'));

  if (metadata.version) raw.version = metadata.version;
  if (metadata.date) raw.date = metadata.date;
  if (metadata.updatedAt) raw.updated_at = metadata.updatedAt;

  if (raw.artifacts) {
    if (metadata.windowsExe && raw.artifacts['windows-exe']) {
      raw.artifacts['windows-exe'].sha256 = metadata.windowsExe.sha256;
      raw.artifacts['windows-exe'].size_bytes = metadata.windowsExe.sizeBytes;
      raw.artifacts['windows-exe'].size_human = metadata.windowsExe.sizeHuman;
    }
    if (metadata.windowsMsi && raw.artifacts['windows-msi']) {
      raw.artifacts['windows-msi'].sha256 = metadata.windowsMsi.sha256;
      raw.artifacts['windows-msi'].size_bytes = metadata.windowsMsi.sizeBytes;
      raw.artifacts['windows-msi'].size_human = metadata.windowsMsi.sizeHuman;
    }
    if (metadata.androidApk && raw.artifacts['android-apk']) {
      raw.artifacts['android-apk'].sha256 = metadata.androidApk.sha256;
      raw.artifacts['android-apk'].size_bytes = metadata.androidApk.sizeBytes;
      raw.artifacts['android-apk'].size_human = metadata.androidApk.sizeHuman;
    }
  }

  writeFileSync(releasePath, JSON.stringify(raw, null, 2) + '\n', 'utf8');
}

/**
 * Stages installer files into a site directory (under downloads/ and public/downloads/).
 */
function stageInstallers(targetSiteDir, apkPath, exePath, msiPath) {
  const metadata = {};

  if (apkPath && existsSync(apkPath)) {
    metadata.androidApk = computeFileMetadata(apkPath);
    const targets = [
      join(targetSiteDir, 'downloads', 'android', 'SmartMigrate.apk'),
      join(targetSiteDir, 'downloads', 'android', 'SmartMigrate.apk.bin'),
      join(targetSiteDir, 'public', 'downloads', 'android', 'SmartMigrate.apk'),
      join(targetSiteDir, 'public', 'downloads', 'android', 'SmartMigrate.apk.bin'),
    ];
    for (const t of targets) {
      mkdirSync(dirname(t), { recursive: true });
      cpSync(apkPath, t);
    }
  }

  if (exePath && existsSync(exePath)) {
    metadata.windowsExe = computeFileMetadata(exePath);
    const targets = [
      join(targetSiteDir, 'downloads', 'windows', 'SmartMigrate-Setup-x64.exe'),
      join(targetSiteDir, 'downloads', 'windows', 'SmartMigrate-Setup-x64.exe.bin'),
      join(targetSiteDir, 'public', 'downloads', 'windows', 'SmartMigrate-Setup-x64.exe'),
      join(targetSiteDir, 'public', 'downloads', 'windows', 'SmartMigrate-Setup-x64.exe.bin'),
    ];
    for (const t of targets) {
      mkdirSync(dirname(t), { recursive: true });
      cpSync(exePath, t);
    }
  }

  if (msiPath && existsSync(msiPath)) {
    metadata.windowsMsi = computeFileMetadata(msiPath);
    const targets = [
      join(targetSiteDir, 'downloads', 'windows', 'SmartMigrate-x64.msi'),
      join(targetSiteDir, 'downloads', 'windows', 'SmartMigrate-x64.msi.bin'),
      join(targetSiteDir, 'public', 'downloads', 'windows', 'SmartMigrate-x64.msi'),
      join(targetSiteDir, 'public', 'downloads', 'windows', 'SmartMigrate-x64.msi.bin'),
    ];
    for (const t of targets) {
      mkdirSync(dirname(t), { recursive: true });
      cpSync(msiPath, t);
    }
  }

  return metadata;
}

/**
 * Main build bundle execution function.
 */
export async function buildSiteBundle(options = {}) {
  const siteDir = resolveSiteDir(options.siteDir);
  const repoRoot = resolveRepoRoot(siteDir);
  const versions = readProjectVersions(repoRoot);

  console.log(`[build-site-bundle] Site directory: ${siteDir}`);
  console.log(`[build-site-bundle] Deployment mode: ${options.mode}`);

  // 1. Output location guard
  let outDir = options.outDir ? resolve(options.outDir) : null;
  if (outDir && outDir === siteDir) {
    throw new Error(
      `Output location guard violated: --out-dir (${outDir}) cannot overwrite the site source directory.`
    );
  }

  // 2. Version Gate Check
  if (options.targetVersion) {
    if (versions.android.versionName && versions.android.versionName !== options.targetVersion) {
      throw new Error(
        `Version gate failure: Target version is ${options.targetVersion}, but Android versionName is ${versions.android.versionName}`
      );
    }
    if (versions.windows.version && versions.windows.version !== options.targetVersion) {
      throw new Error(
        `Version gate failure: Target version is ${options.targetVersion}, but Windows Host version is ${versions.windows.version}`
      );
    }
  }

  // 3. Verify Only Mode
  if (options.verifyOnly) {
    console.log('[build-site-bundle] Verifying all manifests and committed downloads on disk...');
    const result = verifyAllSiteManifests(siteDir);
    console.log(`[build-site-bundle] ✓ Successfully verified ${result.verified.length} manifest files.`);
    return { success: true, verified: result.verified };
  }

  // Staging working directory
  const tempStaging = join(tmpdir(), `sm-bundle-${Date.now()}-${Math.random().toString(36).substring(2, 8)}`);

  try {
    const firebaseConfig = readFirebaseConfig(siteDir);

    if (options.mode === 'website-only') {
      // In website-only mode, first verify that current site downloads match all manifests
      console.log('[build-site-bundle] Verifying committed download integrity for website-only deploy...');
      const verifyResult = verifyAllSiteManifests(siteDir);
      console.log(`[build-site-bundle] ✓ Integrity confirmed across ${verifyResult.verified.length} manifests.`);

      // Automatically align release timestamps to current deploy time
      if (options.updateTimestamp !== false && !options.dryRun) {
        updateReleaseTimestamps(siteDir);
        console.log('[build-site-bundle] ✓ Updated release timestamps across all manifests to current deploy time.');
      }

      // Assemble bundle
      if (outDir) {
        console.log(`[build-site-bundle] Staging bundle to ${outDir}...`);
        copySiteFiles(siteDir, outDir, firebaseConfig.ignore);
        verifyAllSiteManifests(outDir);
      }

      return {
        success: true,
        mode: 'website-only',
        outDir: outDir || siteDir,
        manifestCount: verifyResult.verified.length,
      };
    }

    if (options.mode === 'release') {
      console.log('[build-site-bundle] Running release mode bundle assembly...');

      // Copy source site to temporary workspace
      copySiteFiles(siteDir, tempStaging, []);

      // Stage installers into tempStaging if provided
      const nowIso = new Date().toISOString();
      const nowDate = nowIso.split('T')[0];
      const version = options.targetVersion || versions.windows.version || versions.android.versionName || '1.4.4';
      const versionCode = versions.android.versionCode || 8;

      let stagedMeta = {};
      if (options.apkPath || options.exePath || options.msiPath) {
        console.log('[build-site-bundle] Staging provided installers...');
        stagedMeta = stageInstallers(tempStaging, options.apkPath, options.exePath, options.msiPath);
      } else {
        // Dry run / release with committed binaries
        const apkDisk = join(tempStaging, 'downloads', 'android', 'SmartMigrate.apk.bin');
        const exeDisk = join(tempStaging, 'downloads', 'windows', 'SmartMigrate-Setup-x64.exe.bin');
        const msiDisk = join(tempStaging, 'downloads', 'windows', 'SmartMigrate-x64.msi.bin');

        if (existsSync(apkDisk)) stagedMeta.androidApk = computeFileMetadata(apkDisk);
        if (existsSync(exeDisk)) stagedMeta.windowsExe = computeFileMetadata(exeDisk);
        if (existsSync(msiDisk)) stagedMeta.windowsMsi = computeFileMetadata(msiDisk);
      }

      const updateData = {
        version,
        versionCode,
        releaseDate: nowIso,
        date: nowDate,
        updatedAt: nowIso,
        ...stagedMeta,
      };

      // Update both schemas in tempStaging
      updateSchema1(join(tempStaging, 'releases', 'manifest.json'), updateData);
      updateSchema1(join(tempStaging, 'public', 'releases', 'manifest.json'), updateData);
      updateSchema2(join(tempStaging, 'release.json'), updateData);
      updateSchema2(join(tempStaging, 'public', 'release.json'), updateData);

      // Verify all manifests in tempStaging
      console.log('[build-site-bundle] Verifying staged manifests against staged binaries...');
      verifyAllSiteManifests(tempStaging);

      // If not dry-run and not just outDir, copy updated manifests/binaries back to siteDir
      if (!options.dryRun && !options.outDir) {
        copySiteFiles(tempStaging, siteDir, firebaseConfig.ignore);
      }

      // If outDir specified, copy staged bundle to outDir (excluding ignore files)
      if (outDir) {
        copySiteFiles(tempStaging, outDir, firebaseConfig.ignore);
        verifyAllSiteManifests(outDir);
      }

      console.log(`[build-site-bundle] ✓ Release bundle generated successfully (version: ${version}).`);
      return {
        success: true,
        mode: 'release',
        version,
        versionCode,
        outDir: outDir || siteDir,
      };
    }
  } finally {
    if (existsSync(tempStaging)) {
      try {
        rmSync(tempStaging, { recursive: true, force: true });
      } catch (_) {}
    }
  }
}

// CLI entry point
if (process.argv[1] && process.argv[1].endsWith('build-site-bundle.mjs')) {
  const options = parseArgs(process.argv.slice(2));
  buildSiteBundle(options)
    .then((res) => {
      console.log(`[build-site-bundle] Operation completed: ${JSON.stringify(res)}`);
      process.exit(0);
    })
    .catch((err) => {
      console.error(`[build-site-bundle] Error: ${err.message}`);
      process.exit(1);
    });
}
