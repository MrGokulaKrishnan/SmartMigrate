import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';

console.log('Testing Release Highlights Sanitization and Manifest Verification...\n');

// 1. Verify Manifests exist and have releaseHighlights array without corrupted characters
const manifestPaths = [
  resolve('apps/website/releases/manifest.json'),
  resolve('apps/website/public/releases/manifest.json'),
];

for (const mPath of manifestPaths) {
  assert(existsSync(mPath), `Manifest file missing at ${mPath}`);
  const content = readFileSync(mPath, 'utf8');
  
  // Ensure no replacement characters or corrupted markers exist in file
  assert(!content.includes('\uFFFD'), `Manifest contains replacement char \\uFFFD: ${mPath}`);
  assert(!content.includes('? '), `Manifest contains suspect '? ' bullet corruption: ${mPath}`);

  const manifest = JSON.parse(content);
  assert(Array.isArray(manifest.releaseHighlights), `Manifest missing releaseHighlights array: ${mPath}`);
  assert(manifest.releaseHighlights.length >= 3, `Expected at least 3 release highlights, got ${manifest.releaseHighlights.length}`);

  for (const item of manifest.releaseHighlights) {
    assert(typeof item === 'string' && item.length > 0, 'Highlight item must be a non-empty string');
    assert(!item.startsWith('•'), `Highlight item should not have bullet prefix: "${item}"`);
    assert(!item.startsWith('-'), `Highlight item should not have dash prefix: "${item}"`);
    assert(!item.includes('\uFFFD'), `Highlight item has replacement char: "${item}"`);
  }
  console.log(`✓ Verified manifest integrity: ${mPath}`);
}

// 2. Test Parser Regex used in React & Rust fallback
function sanitizeHighlightLine(line) {
  return line.replace(/^[\s•\-\*\u2022\uFFFD\?]+\s*/, '').trim();
}

const testCases = [
  {
    input: '• Direct LAN throughput accelerated by 40% with zero memory copying.',
    expected: 'Direct LAN throughput accelerated by 40% with zero memory copying.',
  },
  {
    input: '- Cryptographic peer verification using Ed25519 & DPAPI-backed tokens.',
    expected: 'Cryptographic peer verification using Ed25519 & DPAPI-backed tokens.',
  },
  {
    input: '* Added automatic LAN fallback to TURN relay when direct P2P is blocked.',
    expected: 'Added automatic LAN fallback to TURN relay when direct P2P is blocked.',
  },
  {
    input: '? Resolved "os error 740" elevation failure during update installation.',
    expected: 'Resolved "os error 740" elevation failure during update installation.',
  },
  {
    input: '   \t •    Leading spaces and tabs before bullet are removed cleanly.  ',
    expected: 'Leading spaces and tabs before bullet are removed cleanly.',
  },
  {
    input: 'Smart Migrate v1.4.1 without any bullet prefix is preserved.',
    expected: 'Smart Migrate v1.4.1 without any bullet prefix is preserved.',
  },
  {
    input: 'Fix for user\'s "smart-migrated" (LAN/P2P) sync.',
    expected: 'Fix for user\'s "smart-migrated" (LAN/P2P) sync.',
  },
];

for (const tc of testCases) {
  const result = sanitizeHighlightLine(tc.input);
  assert.equal(result, tc.expected, `Failed for input: "${tc.input}"`);
}
console.log('✓ Verified sanitizeHighlightLine against all edge cases (Unicode bullets, OEM corruption, quotes, dashes).');

console.log('\nAll release notes & manifest regression tests passed successfully!');
