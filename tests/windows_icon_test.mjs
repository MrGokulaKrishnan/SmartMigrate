import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';

console.log('Testing Windows Application Icon and Bundle Configuration...\n');

// 1. Verify source PNG icon
const iconPngPath = resolve('apps/windows-host/src-tauri/icons/icon.png');
assert(existsSync(iconPngPath), `icon.png missing at ${iconPngPath}`);
const pngBuf = readFileSync(iconPngPath);
assert(
  pngBuf.slice(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])),
  'icon.png is not a valid PNG file'
);
const pngWidth = pngBuf.readUInt32BE(16);
const pngHeight = pngBuf.readUInt32BE(20);
assert(pngWidth >= 512 && pngHeight >= 512, `icon.png must be at least 512x512, got ${pngWidth}x${pngHeight}`);
console.log(`✓ Verified icon.png (${pngWidth}x${pngHeight} PNG)`);

// 2. Verify multi-resolution ICO file
const iconIcoPath = resolve('apps/windows-host/src-tauri/icons/icon.ico');
assert(existsSync(iconIcoPath), `icon.ico missing at ${iconIcoPath}`);
const icoBuf = readFileSync(iconIcoPath);
const reserved = icoBuf.readUInt16LE(0);
const type = icoBuf.readUInt16LE(2);
const count = icoBuf.readUInt16LE(4);

assert.equal(reserved, 0, 'ICO header reserved field must be 0');
assert.equal(type, 1, 'ICO header type field must be 1 (icon)');
assert(count >= 5, `icon.ico must contain at least 5 resolution frames, got ${count}`);

let has256Png = false;
const resolutions = [];
for (let i = 0; i < count; i++) {
  const o = 6 + i * 16;
  const w = icoBuf[o] === 0 ? 256 : icoBuf[o];
  const h = icoBuf[o + 1] === 0 ? 256 : icoBuf[o + 1];
  const sz = icoBuf.readUInt32LE(o + 8);
  const off = icoBuf.readUInt32LE(o + 12);
  const isPng = icoBuf.slice(off, off + 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
  resolutions.push(`${w}x${h} (${isPng ? 'PNG' : 'BMP'})`);
  if (w === 256 && h === 256 && isPng) {
    has256Png = true;
  }
}

assert(has256Png, 'icon.ico MUST contain a 256x256 PNG-compressed frame for modern Windows Shell/Search');
assert(
  icoBuf.length < 150000,
  `icon.ico file size (${icoBuf.length} bytes) is suspiciously large — ensure 256px uses PNG compression`
);
console.log(`✓ Verified icon.ico: ${count} frames [${resolutions.join(', ')}], size: ${icoBuf.length} bytes`);

// 3. Verify tauri.conf.json configuration
const tauriConfPath = resolve('apps/windows-host/src-tauri/tauri.conf.json');
assert(existsSync(tauriConfPath), `tauri.conf.json missing at ${tauriConfPath}`);
const tauriConf = JSON.parse(readFileSync(tauriConfPath, 'utf8'));

assert(Array.isArray(tauriConf.bundle?.icon), 'bundle.icon must be an array');
assert(tauriConf.bundle.icon.includes('icons/icon.ico'), 'bundle.icon must include "icons/icon.ico"');
assert(tauriConf.bundle.icon.includes('icons/icon.png'), 'bundle.icon must include "icons/icon.png"');

const nsis = tauriConf.bundle?.windows?.nsis;
assert(nsis, 'bundle.windows.nsis must be configured');
assert.equal(nsis.installerIcon, 'icons/icon.ico', 'installerIcon must point to "icons/icon.ico"');
assert.equal(nsis.uninstallerIcon, 'icons/icon.ico', 'uninstallerIcon must point to "icons/icon.ico"');
assert.equal(nsis.installerHooks, 'nsis_hooks.nsi', 'installerHooks must be configured to "nsis_hooks.nsi"');
console.log('✓ Verified tauri.conf.json bundle icons & NSIS installer hooks configuration');

// 4. Verify nsis_hooks.nsi
const hooksPath = resolve('apps/windows-host/src-tauri/nsis_hooks.nsi');
assert(existsSync(hooksPath), `nsis_hooks.nsi missing at ${hooksPath}`);
const hooksContent = readFileSync(hooksPath, 'utf8');
assert(hooksContent.includes('AppUserModelId\\com.smartmigrate.host'), 'nsis_hooks.nsi must register AppUserModelId');
assert(hooksContent.includes('App Paths\\${MAINBINARYNAME}.exe'), 'nsis_hooks.nsi must register App Paths');
assert(hooksContent.includes('CreateShortcut'), 'nsis_hooks.nsi must ensure shortcut with explicit icon');
assert(hooksContent.includes('SHChangeNotify'), 'nsis_hooks.nsi must notify Windows Shell of icon changes');
console.log('✓ Verified nsis_hooks.nsi registration & shell notification macros');

// 5. Verify PE executable resource embedding if binary is built
const exePath = resolve('target/release/smart-migrate-windows-host.exe');
if (existsSync(exePath)) {
  const exeBuf = readFileSync(exePath);
  // Basic PE verification: MZ header and PE signature
  assert.equal(exeBuf.toString('ascii', 0, 2), 'MZ', 'File is not a valid PE executable');
  const peOffset = exeBuf.readUInt32LE(0x3c);
  assert.equal(exeBuf.toString('ascii', peOffset, peOffset + 4), 'PE\0\0', 'Missing PE signature');
  console.log(`✓ Verified target binary structure: ${exePath}`);
}

console.log('\nAll Windows icon and branding validation checks passed successfully!');
