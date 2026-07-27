#!/usr/bin/env node
// Guards ADR-0001: the Cloud Function manifest must stay independent of the
// ecosystem integration manifest, or a broken sibling package can block a deploy again.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const manifestPath = join(repoRoot, 'functions/agents/package.json');

const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));

const DEP_FIELDS = [
  'dependencies',
  'devDependencies',
  'peerDependencies',
  'optionalDependencies',
];

const offenders = DEP_FIELDS.flatMap((field) =>
  Object.keys(manifest[field] ?? {})
    .filter((name) => name.startsWith('@llm-dev-ops/'))
    .map((name) => `${field}.${name}`)
);

if (offenders.length > 0) {
  console.error(
    'ADR-0001 violation: functions/agents/package.json must declare no @llm-dev-ops/* dependency.\n' +
      offenders.map((o) => `  - ${o}`).join('\n') +
      '\nEcosystem dependencies belong in the root package.json, which the Cloud Function does not install.'
  );
  process.exit(1);
}

console.log('ADR-0001 guard passed: functions/agents/package.json declares no @llm-dev-ops/* dependency.');
