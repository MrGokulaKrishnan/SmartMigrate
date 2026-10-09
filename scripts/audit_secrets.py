#!/usr/bin/env python3
import os
import re

PATTERNS = [
    (re.compile(r'(?i)(?:api_key|apikey|secret|password|private_key|token)[\s:=]+[\'"][a-zA-Z0-9_\-\.]{16,}[\'"]'), 'High-entropy secret/token'),
    (re.compile(r'-----BEGIN (?:RSA|OPENSSH|EC|PRIVATE) KEY-----'), 'Private Key Header'),
    (re.compile(r'AIza[0-9A-Za-z-_]{35}'), 'Google API Key'),
    (re.compile(r'ghp_[0-9a-zA-Z]{36}'), 'GitHub Personal Access Token'),
    (re.compile(r'(?i)password\s*=\s*[\'"][^\'"]+[\'"]'), 'Hardcoded password'),
]

ROOT = r"C:\Smart Migrate"
IGNORE_DIRS = {'.git', 'target', 'node_modules', '.gradle', 'build', 'dist', '.system_generated'}

findings = []
for dirpath, dirnames, filenames in os.walk(ROOT):
    dirnames[:] = [d for d in dirnames if d not in IGNORE_DIRS]
    for fname in filenames:
        ext = os.path.splitext(fname)[1].lower()
        if ext in ['.exe', '.apk', '.msi', '.png', '.jpg', '.ico', '.jar', '.lock', '.bin', '.bmp']:
            continue
        fpath = os.path.join(dirpath, fname)
        try:
            with open(fpath, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()
                for pat, label in PATTERNS:
                    for m in pat.finditer(content):
                        start_line = content.rfind('\n', 0, m.start()) + 1
                        end_line = content.find('\n', m.end())
                        if end_line == -1: end_line = len(content)
                        line_str = content[start_line:end_line]
                        if 'publickeytoken' in line_str.lower():
                            continue
                        line_no = content[:m.start()].count('\n') + 1
                        findings.append((fpath, line_no, label, m.group(0)[:60]))
        except Exception:
            pass

print(f"Total secret pattern matches: {len(findings)}")
for fpath, lno, label, preview in findings:
    print(f"  {fpath}:{lno} [{label}] -> {preview}")
