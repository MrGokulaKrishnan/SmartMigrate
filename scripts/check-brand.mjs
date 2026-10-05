import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";

const excluded = new Set([".git", "node_modules", "dist", "build", "target"]);
const legacyWord = ["smart", "bar"].join("\\s*");
const misspelledWord = ["smart", "migarte"].join("\\s*");
const forbidden = new RegExp(`${legacyWord}|${misspelledWord}`, "i");
const textExtensions = new Set([".md", ".txt", ".json", ".yaml", ".yml", ".toml", ".css", ".js", ".mjs", ".rs", ".proto"]);

async function visit(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const failures = [];
  for (const entry of entries) {
    if (excluded.has(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      failures.push(...await visit(path));
      continue;
    }
    const extension = entry.name.slice(entry.name.lastIndexOf("."));
    if (!textExtensions.has(extension)) continue;
    const content = await readFile(path, "utf8");
    if (forbidden.test(content)) failures.push(path);
  }
  return failures;
}

const failures = await visit(process.cwd());
if (failures.length) {
  console.error("Legacy product references found:\n" + failures.join("\n"));
  process.exitCode = 1;
} else {
  console.log("Brand check passed: Smart Migrate naming is consistent.");
}
