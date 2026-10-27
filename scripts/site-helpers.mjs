/**
 * Smart Migrate — Shared Website, Manifest & Firebase Emulation Helpers
 *
 * Implements pure Node 22 utilities with no external dependencies:
 * - Reads firebase.json (rewrites, custom headers, ignore rules)
 * - Validates and updates both manifest schemas:
 *     Schema 1: releases/manifest.json (platforms block: windows-x86_64, android-arm64)
 *     Schema 2: release.json (artifacts block: windows-exe, windows-msi, android-apk)
 * - Cryptographic SHA-256 calculation and byte-for-byte integrity checks
 * - Project version discovery from Android Gradle and Tauri configurations
 */

import { readFileSync, writeFileSync, existsSync, statSync, readdirSync } from 'node:fs';
import { resolve, join, relative, dirname } from 'node:path';
import { createHash } from 'node:crypto';

/**
 * Resolves repository root directory.
 */
export function resolveRepoRoot(startDir = process.cwd()) {
  let curr = resolve(startDir);
  while (curr !== dirname(curr)) {
    if (existsSync(join(curr, 'package.json')) && existsSync(join(curr, 'apps'))) {
      return curr;
    }
    curr = dirname(curr);
  }
  return resolve(startDir);
}

/**
 * Resolves website directory, honoring SMART_MIGRATE_SITE_DIR override.
 */
export function resolveSiteDir(customDir) {
  if (customDir && existsSync(customDir)) {
    return resolve(customDir);
  }
  if (process.env.SMART_MIGRATE_SITE_DIR && existsSync(process.env.SMART_MIGRATE_SITE_DIR)) {
    return resolve(process.env.SMART_MIGRATE_SITE_DIR);
  }
  const root = resolveRepoRoot();
  const defaultSite = join(root, 'apps', 'website');
  if (existsSync(defaultSite)) {
    return defaultSite;
  }
  return root;
}

/**
 * Computes SHA-256 hex digest for a file.
 */
export function computeFileSha256(filePath) {
  if (!existsSync(filePath)) {
    throw new Error(`Cannot compute SHA-256: file not found at "${filePath}"`);
  }
  const buf = readFileSync(filePath);
  return createHash('sha256').update(buf).digest('hex');
}

/**
 * Formats byte size into human readable string (e.g. "4.0 MB").
 */
export function formatFileSize(bytes) {
  if (typeof bytes !== 'number' || isNaN(bytes)) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Returns file metadata: size in bytes, human size, and SHA-256.
 */
export function computeFileMetadata(filePath) {
  const stat = statSync(filePath);
  const sha256 = computeFileSha256(filePath);
  return {
    sizeBytes: stat.size,
    sizeHuman: formatFileSize(stat.size),
    sha256,
  };
}

/**
 * Reads and normalizes firebase.json configuration.
 */
export function readFirebaseConfig(siteDir) {
  const configPath = join(siteDir, 'firebase.json');
  if (!existsSync(configPath)) {
    return { hosting: [], rewrites: [], headers: [], ignore: [] };
  }
  const raw = JSON.parse(readFileSync(configPath, 'utf8'));
  const hostingList = Array.isArray(raw.hosting) ? raw.hosting : raw.hosting ? [raw.hosting] : [];

  // Merge rewrites, headers, and ignore rules from all site blocks
  const rewrites = [];
  const headers = [];
  const ignore = [];

  for (const block of hostingList) {
    if (Array.isArray(block.rewrites)) rewrites.push(...block.rewrites);
    if (Array.isArray(block.headers)) headers.push(...block.headers);
    if (Array.isArray(block.ignore)) ignore.push(...block.ignore);
  }

  return {
    raw,
    hosting: hostingList,
    rewrites,
    headers,
    ignore,
  };
}

/**
 * Checks if a relative path matches simple glob patterns:
 * - "**\/*.apk"
 * - "**\/. *"
 * - "**\/node_modules\/**"
 */
export function matchesGlob(filePath, pattern) {
  const normalizedPath = filePath.replace(/\\/g, '/').replace(/^\//, '');
  const normalizedPattern = pattern.replace(/\\/g, '/');

  if (normalizedPattern === '**') return true;

  // Exact file name
  if (normalizedPattern === normalizedPath) return true;

  // Simple wildcard conversion to regex
  const regexStr = '^' + normalizedPattern
    .replace(/[.+^${}()|[\]\\]/g, '\\$&')
    .replace(/\*\*\/\*/g, '.*')
    .replace(/\*\*/g, '.*')
    .replace(/\*/g, '[^/]*') + '$';

  const regex = new RegExp(regexStr);
  return regex.test(normalizedPath) || regex.test('/' + normalizedPath);
}

/**
 * Resolves a request path through firebase.json rewrites.
 */
export function resolveRewrite(urlPath, rewrites = []) {
  const clean = urlPath.split('?')[0].replace(/\/+$/, '') || '/';
  for (const r of rewrites) {
    if (r.source === clean) {
      return r.destination;
    }
  }
  return clean;
}

/**
 * Resolves headers for a request path through firebase.json headers rules.
 */
export function resolveHeaders(urlPath, headersList = []) {
  const clean = urlPath.split('?')[0];
  const merged = {};

  for (const rule of headersList) {
    let matches = false;
    if (rule.source === '**') {
      matches = true;
    } else if (rule.source === clean || rule.source === clean.replace(/\/+$/, '')) {
      matches = true;
    } else if (matchesGlob(clean, rule.source)) {
      matches = true;
    }

    if (matches && Array.isArray(rule.headers)) {
      for (const h of rule.headers) {
        merged[h.key] = h.value;
      }
    }
  }

  return merged;
}

/**
 * Detects schema of manifest data:
 * - 'platforms' for Schema 1 (manifest.json)
 * - 'artifacts' for Schema 2 (release.json)
 * - null for unknown
 */
export function detectSchema(data) {
  if (!data || typeof data !== 'object') return null;
  if (data.platforms && typeof data.platforms === 'object') return 'platforms';
  if (data.artifacts && typeof data.artifacts === 'object') return 'artifacts';
  return null;
}

/**
 * Finds artifact binary on disk under siteDir, checking both direct path and .bin alias.
 */
export function findArtifactOnDisk(siteDir, downloadSubpath) {
  const normalized = downloadSubpath.replace(/^\//, '');
  const candidateDirect = join(siteDir, normalized);
  const candidateBin = candidateDirect + '.bin';

  if (existsSync(candidateBin)) {
    return candidateBin;
  }
  if (existsSync(candidateDirect)) {
    return candidateDirect;
  }
  return null;
}

/**
 * Verifies Schema 1 (manifest.json with `platforms` block).
 */
export function verifyManifestSchema1(manifestPath, siteDir) {
  if (!existsSync(manifestPath)) {
    throw new Error(`Manifest missing: ${manifestPath}`);
  }
  const content = readFileSync(manifestPath, 'utf8');
  const manifest = JSON.parse(content);

  if (!manifest.platforms || typeof manifest.platforms !== 'object') {
    throw new Error(`Schema 1 manifest missing "platforms" object: ${manifestPath}`);
  }

  const results = [];

  for (const [platformKey, platformData] of Object.entries(manifest.platforms)) {
    const fileName = platformData.fileName || platformKey;
    const recordedSha = (platformData.sha256 || '').toLowerCase();
    const recordedSize = Number(platformData.size);

    // Expected relative download location
    const subpath = platformKey.startsWith('win')
      ? join('downloads', 'windows', fileName)
      : join('downloads', 'android', fileName);

    const diskPath = findArtifactOnDisk(siteDir, subpath);
    if (!diskPath) {
      throw new Error(
        `Artifact not found on disk for platform "${platformKey}" (${fileName}) in ${siteDir}`
      );
    }

    const { sizeBytes, sha256 } = computeFileMetadata(diskPath);

    if (sha256 !== recordedSha) {
      throw new Error(
        `Manifest SHA-256 mismatch for platform "${platformKey}" in ${manifestPath}:\n` +
        `  Recorded in manifest: ${recordedSha}\n` +
        `  Actual file on disk:  ${sha256}\n` +
        `  File path:            ${diskPath}`
      );
    }

    if (sizeBytes !== recordedSize) {
      throw new Error(
        `Manifest size mismatch for platform "${platformKey}" in ${manifestPath}:\n` +
        `  Recorded in manifest: ${recordedSize} bytes\n` +
        `  Actual file on disk:  ${sizeBytes} bytes\n` +
        `  File path:            ${diskPath}`
      );
    }

    results.push({ platform: platformKey, fileName, diskPath, sha256, sizeBytes });
  }

  return { valid: true, schema: 'platforms', results };
}

/**
 * Verifies Schema 2 (release.json with `artifacts` block).
 */
export function verifyReleaseSchema2(releasePath, siteDir) {
  if (!existsSync(releasePath)) {
    throw new Error(`Release JSON missing: ${releasePath}`);
  }
  const content = readFileSync(releasePath, 'utf8');
  const release = JSON.parse(content);

  if (!release.artifacts || typeof release.artifacts !== 'object') {
    throw new Error(`Schema 2 missing "artifacts" object: ${releasePath}`);
  }

  const results = [];

  for (const [artifactKey, artifactData] of Object.entries(release.artifacts)) {
    const recordedSha = (artifactData.sha256 || '').toLowerCase();
    const recordedSize = Number(artifactData.size_bytes);
    const downloadUrl = artifactData.download_url || artifactData.path || '';

    const diskPath = findArtifactOnDisk(siteDir, downloadUrl);
    if (!diskPath) {
      throw new Error(
        `Artifact not found on disk for artifact key "${artifactKey}" (${downloadUrl}) in ${siteDir}`
      );
    }

    const { sizeBytes, sha256 } = computeFileMetadata(diskPath);

    if (sha256 !== recordedSha) {
      throw new Error(
        `Release JSON SHA-256 mismatch for artifact "${artifactKey}" in ${releasePath}:\n` +
        `  Recorded in JSON:    ${recordedSha}\n` +
        `  Actual file on disk: ${sha256}\n` +
        `  File path:           ${diskPath}`
      );
    }

    if (sizeBytes !== recordedSize) {
      throw new Error(
        `Release JSON size mismatch for artifact "${artifactKey}" in ${releasePath}:\n` +
        `  Recorded in JSON:    ${recordedSize} bytes\n` +
        `  Actual file on disk: ${sizeBytes} bytes\n` +
        `  File path:           ${diskPath}`
      );
    }

    results.push({ artifact: artifactKey, diskPath, sha256, sizeBytes });
  }

  return { valid: true, schema: 'artifacts', results };
}

/**
 * Discovers and verifies all manifest copies in a site directory.
 */
export function verifyAllSiteManifests(siteDir) {
  const verified = [];

  // Schema 1 locations
  const schema1Candidates = [
    join(siteDir, 'releases', 'manifest.json'),
    join(siteDir, 'public', 'releases', 'manifest.json'),
  ];
  for (const p of schema1Candidates) {
    if (existsSync(p)) {
      verifyManifestSchema1(p, siteDir);
      verified.push({ path: p, schema: 'platforms' });
    }
  }

  // Schema 2 locations
  const schema2Candidates = [
    join(siteDir, 'release.json'),
    join(siteDir, 'public', 'release.json'),
  ];
  for (const p of schema2Candidates) {
    if (existsSync(p)) {
      verifyReleaseSchema2(p, siteDir);
      verified.push({ path: p, schema: 'artifacts' });
    }
  }

  if (verified.length === 0) {
    throw new Error(`No release manifests found to verify in "${siteDir}"`);
  }

  return { valid: true, verified };
}

/**
 * Reads project versions from Android build.gradle.kts and Windows tauri.conf.json.
 */
export function readProjectVersions(repoRoot = resolveRepoRoot()) {
  const gradlePath = join(repoRoot, 'apps', 'android-client', 'app', 'build.gradle.kts');
  const tauriPath = join(repoRoot, 'apps', 'windows-host', 'src-tauri', 'tauri.conf.json');

  let android = { versionName: null, versionCode: null };
  if (existsSync(gradlePath)) {
    const gradle = readFileSync(gradlePath, 'utf8');
    const nameMatch = gradle.match(/versionName\s*=\s*"([^"]+)"/);
    const codeMatch = gradle.match(/versionCode\s*=\s*(\d+)/);
    if (nameMatch) android.versionName = nameMatch[1];
    if (codeMatch) android.versionCode = parseInt(codeMatch[1], 10);
  }

  let windows = { version: null };
  if (existsSync(tauriPath)) {
    const tauri = JSON.parse(readFileSync(tauriPath, 'utf8'));
    if (tauri.version) windows.version = tauri.version;
  }

  return { android, windows, repoRoot };
}

/**
 * Updates releaseDate, updated_at, and date across all manifest copies in siteDir.
 */
export function updateReleaseTimestamps(siteDir = resolveSiteDir(), timestamp = new Date().toISOString()) {
  const dateOnly = timestamp.split('T')[0];
  const updatedFiles = [];

  // Schema 1 paths
  const schema1Paths = [
    join(siteDir, 'releases', 'manifest.json'),
    join(siteDir, 'public', 'releases', 'manifest.json'),
  ];
  for (const p of schema1Paths) {
    if (existsSync(p)) {
      const raw = JSON.parse(readFileSync(p, 'utf8'));
      raw.releaseDate = timestamp;
      writeFileSync(p, JSON.stringify(raw, null, 2) + '\n', 'utf8');
      updatedFiles.push(p);
    }
  }

  // Schema 2 paths
  const schema2Paths = [
    join(siteDir, 'release.json'),
    join(siteDir, 'public', 'release.json'),
  ];
  for (const p of schema2Paths) {
    if (existsSync(p)) {
      const raw = JSON.parse(readFileSync(p, 'utf8'));
      raw.date = dateOnly;
      raw.updated_at = timestamp;
      writeFileSync(p, JSON.stringify(raw, null, 2) + '\n', 'utf8');
      updatedFiles.push(p);
    }
  }

  return { success: true, timestamp, dateOnly, updatedFiles };
}
