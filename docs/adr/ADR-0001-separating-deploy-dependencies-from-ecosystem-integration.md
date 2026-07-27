# ADR-0001: Separating Deploy-Time Dependencies from Ecosystem Integration

**Status:** Proposed
**Date:** 2026-07-27
**Decision Makers:** benchmark-exchange maintainers, LLM-Dev-Ops infra owners
**Context for:** commit `607d9fc` (HEAD)

## Context

The declared LLM-Dev-Ops ecosystem integration for `@llm-dev-ops/benchmark-exchange`
does not function. It was deleted to make the Cloud Function deploy succeed, and
the metadata describing it was left in place.

### What the head commit did

`git -C /workspace/agentics-dev/benchmark-exchange log --oneline -15` puts this
at HEAD:

```
607d9fc Remove unresolvable @llm-dev-ops deps from package.json for CF deploy
```

Full message:

> Remove unresolvable @llm-dev-ops deps from package.json for CF deploy
>
> Cloud Function uses only Node.js builtins (crypto). The @llm-dev-ops/*
> packages are ecosystem metadata and do not exist on the npm registry,
> causing npm install to fail during Cloud Build.

The diff (`+1 −19`) removed the entire `dependencies`, `peerDependencies`, and
`devDependencies` blocks:

```diff
-  "dependencies": {
-    "@llm-dev-ops/llm-marketplace-model-marketplace": "^1.1.1",
-    "@llm-dev-ops/llm-marketplace-sdk": "^1.1.1",
-    "@llm-dev-ops/infra-config": "^0.2.0",
-    "@llm-dev-ops/infra-logging": "^0.2.0",
-    "@llm-dev-ops/infra-tracing": "^0.2.0",
-    "@llm-dev-ops/infra-errors": "^0.2.0",
-    "@llm-dev-ops/infra-cache": "^0.2.0",
-    "@llm-dev-ops/infra-retry": "^0.2.0",
-    "@llm-dev-ops/infra-ratelimit": "^0.2.0",
-    "claude-code": "^1.0.0",
-    "claude-flow": "^2.7.41"
-  },
-  "peerDependencies": {
-    "@llm-dev-ops/infra-core": "^0.2.0"
-  },
-  "devDependencies": {
-    "@llm-dev-ops/infra-testing": "^0.2.0"
-  },
+  "dependencies": {},
```

### What was left behind

`package.json` still carries the ecosystem contract that those dependencies were
supposed to implement:

```json
"llmDevOps": {
  "phase": "2B",
  "infraIntegration": {
    "version": "0.2.0",
    "modules": ["config","logging","tracing","errors","cache","retry","ratelimit"],
    "featureFlags": { "infraIntegration": true, "legacyLocal": false }
  },
  "exposesTo": ["llm-test-bench","llm-analytics-hub","llm-registry","llm-gateway"],
  "consumesFrom": ["llm-infra","llm-registry","llm-observatory","llm-marketplace"]
}
```

`featureFlags.infraIntegration` is `true` and `legacyLocal` is `false` while
`dependencies` is `{}`. The manifest asserts that seven infra modules are wired
in and that no local fallback is in use. Neither is true. `DEPENDENCIES.md:3-7`
compounds this, still declaring **"Status: Complete"** for the Phase 2B infra
integration and listing all removed packages in a dependency table at
`DEPENDENCIES.md:93-101`.

### The finding the commit message understates

The commit says the packages "do not exist on the npm registry." Verified
against this workspace, the situation is worse than a publishing gap — **eight
of the eleven removed packages have no source anywhere in the ecosystem, under
any name.** They were never buildable, so no amount of publishing work would
have resolved them.

`grep -rl '"name": "@llm-dev-ops/infra-' --include=package.json .` across the
whole workspace returns **no matches**. The `infra` repo is a Rust project
(`infra/Cargo.toml`, `infra/rust-toolchain.toml`, `infra/crates/`). It publishes
Rust crates — `infra/crates/` contains `infra-config`, `infra-cache`,
`infra-retry`, `infra-errors`, `infra-rate-limit`, `infra-otel`, and 13 others,
each declaring an unscoped Cargo package name (`name = "infra-config"`, etc.).

`infra` publishes exactly **one** npm package, at `infra/sdk/ts/package.json`:

```json
{ "name": "@llm-dev-ops/infra", "version": "0.1.0" }
```

It is a single package with subpath exports — `./crypto`, `./id`, `./json`,
`./llm-client`, `./retry`, `./cache`, `./rate-limit` — not seven sibling
packages. So the removed declarations were wrong on every axis:

| Declared | Reality |
|---|---|
| `@llm-dev-ops/infra-config@^0.2.0` | no such npm package; nearest is the `infra-config` **Rust crate** |
| `@llm-dev-ops/infra-cache@^0.2.0` | subpath `@llm-dev-ops/infra@0.1.0` → `./cache` |
| `@llm-dev-ops/infra-retry@^0.2.0` | subpath `@llm-dev-ops/infra@0.1.0` → `./retry` |
| `@llm-dev-ops/infra-ratelimit@^0.2.0` | subpath `@llm-dev-ops/infra@0.1.0` → `./rate-limit` |
| `@llm-dev-ops/infra-errors@^0.2.0` | `errors.ts` exists in the SDK but is not an export subpath |
| `@llm-dev-ops/infra-logging@^0.2.0` | no TS equivalent; Rust `infra-otel` only |
| `@llm-dev-ops/infra-tracing@^0.2.0` | no TS equivalent; Rust `infra-otel` only |
| `@llm-dev-ops/infra-core@^0.2.0` (peer) | no such package |
| `@llm-dev-ops/infra-testing@^0.2.0` (dev) | no such package |

The version was wrong too: every declaration pinned `^0.2.0`; the real package
is `0.1.0`, so `^0.2.0` would not match it even after publication.

By contrast, the two marketplace dependencies were **real and correctly
versioned**. `marketplace/sdks/javascript/package.json:2-3` declares
`@llm-dev-ops/llm-marketplace-sdk@1.1.1` and
`marketplace/services/model-marketplace/package.json:2-3` declares
`@llm-dev-ops/llm-marketplace-model-marketplace@1.1.1` — both satisfying the
removed `^1.1.1` ranges, and both carrying
`publishConfig: { access: "public", registry: "https://registry.npmjs.org/" }`.
These two were collateral damage: they were deleted along with the fictional
ones because the install failed as a batch.

The remaining two, `claude-code@^1.0.0` and `claude-flow@^2.7.41`, are developer
tooling. They have no business in the runtime `dependencies` of a Cloud
Function regardless of whether they resolve.

### Why the deploy broke in the first place

The repository is dual-stack. Its primary artifact is Rust — `Cargo.toml`,
`crates/`, `sqlx.toml`, and `cloudbuild.yaml`, which runs
`cargo test --workspace --release` and builds `llm-benchmark-api` for Cloud Run.
Commit `652558b` then added a Node.js Cloud Function at the repository **root**
(`index.js`, `SERVICE_NAME = 'benchmark-exchange-agents'`), whose only import is
`const crypto = require('crypto')`.

Because that function lives at the root, the Cloud Function deploy packages the
root directory and `npm install`s the root `package.json`. That one file was
therefore serving two incompatible jobs at once:

1. **the install manifest** for a zero-dependency Cloud Function, and
2. **the ecosystem integration declaration** for the wider platform.

Job 2 broke job 1. With no lockfile committed (there is no `package-lock.json`
in the repository root), every Cloud Build ran an unconstrained resolution
against the public registry, so a single unresolvable name failed the deploy.
Emptying `dependencies` fixed the deploy by discarding job 2 entirely.

That is the root cause: **one manifest, two audiences.** Any fix that puts the
ecosystem dependencies back into the root `package.json` re-breaks the deploy.

## Decision

We will **separate the deployment manifest from the integration manifest**, and
we will **correct the dependency names rather than restore them**. Specifically:

1. **Relocate the Cloud Function into `functions/agents/`** with its own
   `package.json` declaring `"dependencies": {}` and its own committed
   `package-lock.json`. The CF deploy is repointed at that directory. Its
   dependency set becomes structurally independent of the ecosystem.
2. **Restore ecosystem dependencies to the root `package.json` under corrected
   names and exact pins** — never carets. The root manifest becomes the
   integration declaration and is not what the CF deploy installs.
3. **Declare only what resolves.** The eight fictional `@llm-dev-ops/infra-*`
   entries are **not** restored. They are replaced by the single real package,
   `"@llm-dev-ops/infra": "0.1.0"`, consumed via its subpath exports. The two
   marketplace packages are restored at exact `1.1.1`.
4. **A publish gate blocks re-introduction of unresolvable names.** CI runs
   `npm install` against the root manifest on every PR. A dependency that cannot
   be installed fails the build *there*, in the PR, rather than in a deploy.
5. **`claude-code` and `claude-flow` are not restored** as runtime dependencies.
   If needed for local workflows they belong in `devDependencies`, and
   `claude-code` needs its real registry name verified before it is added back
   at all.
6. **Metadata must match reality.** `llmDevOps.infraIntegration` is corrected to
   `version: "0.1.0"`, its `modules` array reduced to modules that actually have
   an export subpath, and `featureFlags.infraIntegration` set to whatever is
   *currently* true — `false` until step 2 lands, `true` after.

### Rejected alternatives

- **Pin the existing names to specific published versions.** This was the
  survey's suggested fix and it does not work: eight of the names have no source
  and no publishable artifact. There is no version to pin them to. This is the
  single most important correction in this ADR.
- **Publish seven `@llm-dev-ops/infra-*` packages to match the declarations.**
  Rewrites the `infra` repo's public surface to match a consumer's mistaken
  assumption. The `infra` team chose one package with subpath exports; the
  consumer should adapt.
- **Keep `dependencies: {}` and delete the `llmDevOps` block.** Honest and
  cheap, but abandons the integration and leaves `consumesFrom` unimplemented
  across the platform. Rejected as a goal, though it is strictly better than the
  status quo and is the correct fallback if step 2 stalls.
- **`npm install --omit=optional` / `--no-optional` at deploy.** Papers over the
  coupling without removing it, and still fails on non-optional entries.

## Consequences

### Positive

- A broken or unpublished sibling package can no longer block a production
  deploy. The two concerns are separated by directory, not by discipline.
- The manifest stops lying. `featureFlags.infraIntegration: true` becomes a
  verifiable claim.
- Exact pins plus a committed lockfile make Cloud Build reproducible; today's
  unconstrained resolution can drift between two runs of the same commit.
- Unresolvable dependencies surface in PR CI, where they are cheap, instead of
  in a deploy, where they are an outage.
- The two legitimate marketplace integrations, deleted only as collateral, come
  back.

### Negative

- Moving `index.js` changes the Cloud Function's deploy path and source
  directory. The deploy configuration and any `gcloud functions deploy`
  invocation must be updated in the same change, and a mismatch means a failed
  or — worse — a stale-but-successful deploy.
- Only 3 of the 11 original dependencies come back. The infra integration
  narrows to what `@llm-dev-ops/infra@0.1.0` genuinely exports; `logging` and
  `tracing` have no TypeScript implementation at all, so those two modules
  remain unimplemented until the `infra` team ships them or this repo talks to
  the Rust `infra-otel` crate through another channel.
- Exact pins mean upgrades are explicit work.

### Neutral

- The repo keeps two `package.json` files with different purposes. This must be
  documented or a future contributor will re-add an ecosystem dependency to the
  function manifest and reintroduce the coupling.
- The Rust/Cloud Run path (`cloudbuild.yaml`, `crates/`) is untouched.

## Implementation Plan

1. **Confirm publication status of the three real packages.** Run
   `npm view @llm-dev-ops/infra@0.1.0`,
   `npm view @llm-dev-ops/llm-marketplace-sdk@1.1.1`, and
   `npm view @llm-dev-ops/llm-marketplace-model-marketplace@1.1.1`. Any that is
   unpublished becomes a blocking upstream request before step 5. Source
   existing is not the same as published.
2. **Create `functions/agents/`** and move `index.js` and `test.js` into it
   unchanged. Add `functions/agents/package.json` with
   `"dependencies": {}`, `"engines": { "node": ">=20.0.0" }`, and
   `"main": "index.js"`.
3. **Generate and commit `functions/agents/package-lock.json`** so the deploy
   installs a fixed graph.
4. **Repoint the Cloud Function deploy** at `functions/agents/`. Deploy to a
   staging function name first and confirm the handler responds before touching
   the production deploy.
5. **Restore corrected dependencies to the root `package.json`:**
   ```json
   "dependencies": {
     "@llm-dev-ops/infra": "0.1.0",
     "@llm-dev-ops/llm-marketplace-sdk": "1.1.1",
     "@llm-dev-ops/llm-marketplace-model-marketplace": "1.1.1"
   }
   ```
   Commit the resulting root `package-lock.json`.
6. **Correct the `llmDevOps` block** — set `infraIntegration.version` to
   `"0.1.0"`, reduce `modules` to those with real export subpaths
   (`cache`, `retry`, `ratelimit`, plus `crypto`/`id`/`json` if used), and drop
   `logging`/`tracing`/`config` until an implementation exists.
7. **Write the integration code** that imports the subpaths
   (`@llm-dev-ops/infra/retry`, `/cache`, `/rate-limit`). Only once this exists
   and is exercised by a test may `featureFlags.infraIntegration` be set `true`.
   Until then it is `false` and `legacyLocal` is `true`.
8. **Add the CI publish gate** — a PR job running `npm ci` at the root and
   `npm ci` in `functions/agents/`, both required to pass.
9. **Add a CI guard against recoupling** — assert that
   `functions/agents/package.json` declares no `@llm-dev-ops/*` dependency, so
   the separation cannot be silently undone.
10. **Rewrite `DEPENDENCIES.md`** — the Phase 2B "Status: Complete" claim at
    lines 3-7 and the package table at lines 93-101 are false as written.
    Replace with the corrected set and link this ADR.
11. **File an upstream issue against `infra`** asking whether TypeScript
    `logging`/`tracing` equivalents of the `infra-otel` crate are planned, so
    the gap in step 6 is tracked rather than forgotten.

Steps 2-4 are independently valuable and unblock nothing else — they can ship
first and alone. Step 5 depends on step 1 resolving favourably.

## Verification

The decision is implemented when all of the following hold:

1. `functions/agents/package.json` exists, declares `"dependencies": {}`, and
   `functions/agents/package-lock.json` is committed.
2. `index.js` no longer exists at the repository root.
3. `npm ci` inside `functions/agents/` succeeds with zero network resolution of
   any `@llm-dev-ops/*` name.
4. The Cloud Function deploy runs green against `functions/agents/`, and the
   deployed handler returns a valid response for a `publish` agent request.
5. `npm ci` at the repository root succeeds, installing exactly
   `@llm-dev-ops/infra@0.1.0`,
   `@llm-dev-ops/llm-marketplace-sdk@1.1.1`, and
   `@llm-dev-ops/llm-marketplace-model-marketplace@1.1.1`.
6. `grep -c '\^' package.json` returns `0` for the `dependencies` block — no
   caret ranges on ecosystem packages.
7. `grep -n 'infra-config\|infra-logging\|infra-tracing\|infra-core\|infra-testing' package.json`
   returns nothing — the fictional names are gone and have not crept back.
8. `llmDevOps.infraIntegration.version` equals the installed version of
   `@llm-dev-ops/infra`, verifiable by comparing against `package-lock.json`.
9. `featureFlags.infraIntegration` is `true` **only if** at least one
   `require`/`import` of an `@llm-dev-ops/infra` subpath exists in shipped code
   and is covered by a passing test.
10. The CI recoupling guard (step 9) fails when an `@llm-dev-ops/*` dependency
    is deliberately added to `functions/agents/package.json` — test the guard by
    breaking it once.
11. `DEPENDENCIES.md` contains no reference to a package that `npm view` cannot
    resolve.

Check 7 is the one that distinguishes this fix from the survey's proposed
"pin the versions" approach, and check 10 is what stops the root cause from
recurring.
