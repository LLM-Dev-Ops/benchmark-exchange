# LLM Benchmark Exchange - Dependency Documentation

## Phase 2B Infra Integration Status

**Status:** Not integrated — blocked on upstream publication
**Last verified:** 2026-07-27
**Governing decision:** [ADR-0001: Separating Deploy-Time Dependencies from Ecosystem Integration](docs/adr/ADR-0001-separating-deploy-dependencies-from-ecosystem-integration.md)

An earlier revision of this document declared Phase 2B "Complete" against
`@llm-dev-ops/infra-*@^0.2.0` npm packages. Those packages do not exist. Eight of
the nine declared names have no source anywhere in the ecosystem under any name,
and the version range was wrong for the one that does. The declarations were
removed from `package.json` in commit `607d9fc` to unblock the Cloud Function
deploy; this document now reflects what is actually wired in.

`llmDevOps.infraIntegration.featureFlags.infraIntegration` is `false` and
`legacyLocal` is `true`. The local implementations under `crates/` are the ones
in use.

---

## Two manifests, two audiences

ADR-0001 separates the deploy manifest from the integration manifest. Adding an
ecosystem dependency to the wrong one re-breaks the Cloud Function deploy.

| Manifest | Purpose | Dependency rule |
|----------|---------|-----------------|
| `package.json` (root) | Ecosystem integration declaration | Exact pins only, never carets. Every entry must resolve on the public registry. |
| `functions/agents/package.json` | Cloud Function `benchmark-exchange-agents` deploy manifest | `dependencies` stays empty. No `@llm-dev-ops/*` entry, ever. |

The Cloud Function is deployed from `functions/agents/`, not from the repository
root — for example `gcloud functions deploy benchmark-exchange-agents
--source=functions/agents --entry-point=handler --runtime=nodejs20`. It imports
only the Node.js builtin `crypto`, and both manifests ship a committed
`package-lock.json` so installs are reproducible.

`scripts/check-function-manifest.mjs` fails if any `@llm-dev-ops/*` dependency
appears in `functions/agents/package.json`. Run it alongside `npm ci` against
both manifests, so an unresolvable dependency fails in a pull request rather
than in a deploy.

> **Pending:** the `node-manifests` CI job that wires those two checks into
> `.github/workflows/ci.yml` is not yet merged — see the pull request
> implementing ADR-0001. Until it lands, the guard is available locally but not
> enforced automatically.

---

## Node.js Packages

### In use

| Package | Version | Registry status | Purpose |
|---------|---------|-----------------|---------|
| `@llm-dev-ops/llm-marketplace-model-marketplace` | `1.1.1` (exact) | Published, verified via `npm view` | Marketplace integration |
| `@llm-dev-ops/llm-marketplace-sdk` | `1.1.1` (exact) | Published, verified via `npm view` | Marketplace SDK |

Both were deleted as collateral damage in `607d9fc` and are restored by
ADR-0001. Their source lives at `marketplace/services/model-marketplace` and
`marketplace/sdks/javascript`.

### Blocked on upstream

| Package | Version | Registry status |
|---------|---------|-----------------|
| `@llm-dev-ops/infra` | `0.1.0` | **Unpublished** — `npm view` returns 404 |

`@llm-dev-ops/infra` is the single real npm package produced by the `infra`
repository (`infra/sdk/ts`). It supersedes the seven fictional
`@llm-dev-ops/infra-*` packages: it exposes its modules as export subpaths of one
package, not as sibling packages. It is **not** declared as a dependency here,
because declaring an unpublished package would break `npm ci` — the exact
failure mode ADR-0001 exists to prevent. It is added once `infra` publishes it.

Its export subpaths are `.`, `./crypto`, `./id`, `./json`, `./llm-client`,
`./retry`, `./cache`, and `./rate-limit`.

### Removed and not returning

| Declared name | Reality |
|---------------|---------|
| `@llm-dev-ops/infra-config@^0.2.0` | No such npm package; nearest is the `infra-config` Rust crate |
| `@llm-dev-ops/infra-cache@^0.2.0` | Subpath `@llm-dev-ops/infra` → `./cache` |
| `@llm-dev-ops/infra-retry@^0.2.0` | Subpath `@llm-dev-ops/infra` → `./retry` |
| `@llm-dev-ops/infra-ratelimit@^0.2.0` | Subpath `@llm-dev-ops/infra` → `./rate-limit` |
| `@llm-dev-ops/infra-errors@^0.2.0` | `errors.ts` exists in the SDK but is not an export subpath |
| `@llm-dev-ops/infra-logging@^0.2.0` | No TypeScript equivalent; Rust `infra-otel` crate only |
| `@llm-dev-ops/infra-tracing@^0.2.0` | No TypeScript equivalent; Rust `infra-otel` crate only |
| `@llm-dev-ops/infra-core@^0.2.0` (peer) | No such package |
| `@llm-dev-ops/infra-testing@^0.2.0` (dev) | No such package |
| `claude-code@^1.0.0` | Developer tooling, not a Cloud Function runtime dependency; real registry name unverified |
| `claude-flow@^2.7.41` | Developer tooling, not a runtime dependency |

`logging` and `tracing` have no TypeScript implementation at all. That gap is
tracked upstream against the `infra` repository and is not closed by ADR-0001.

---

## Rust Crates

The Rust workspace is outside the scope of ADR-0001 and is unchanged. These are
Cargo git dependencies declared in `Cargo.toml`, not npm packages, so the
registry verification above does not apply to them.

| Dependency | Version | Purpose |
|------------|---------|---------|
| `llm-registry-core` | `0.1` | Core registry types and models |
| `llm-registry-service` | `0.1` | Registry service abstractions |
| `llm-registry-api` | `0.1` | Registry API client |
| `llm-observatory-core` | `0.1.1` | Observability types |
| `llm-observatory-sdk` | `0.1.1` | Observability SDK |
| `llm-observatory-collector` | `0.1.1` | Telemetry collection |
| `llm-infra-*` | `0.2` | Phase 2B infra modules — **unverified**, see below |

`Cargo.toml` declares nine `llm-infra-*` crates at version `0.2`. The `infra`
repository's `crates/` directory publishes unscoped names without the `llm-`
prefix (`infra-config`, `infra-cache`, `infra-retry`, `infra-errors`,
`infra-rate-limit`, `infra-otel`, and others). Whether these git dependencies
resolve has not been verified, and the same class of naming mismatch that
produced the npm problem may apply here. Resolving that is separate work; treat
this row as a known open question rather than a confirmed integration.

---

## Phase 1: Exposes-To (What Benchmark Exchange Provides)

| Consumer | Interface | Description |
|----------|-----------|-------------|
| LLM-Test-Bench | Benchmark API | Canonical benchmark definitions for test execution |
| LLM-Analytics-Hub | Results API | Submission results and leaderboard data |
| LLM-Registry | Benchmark Metadata | Benchmark descriptors for model registry |
| LLM-Gateway | Validation API | Benchmark compliance validation endpoints |

---

## Local Implementations

These are the implementations actually in use. They were slated for replacement
by infra modules under the Phase 2B plan; because that integration did not land,
none of them are currently superseded.

| File | Lines | Intended replacement |
|------|-------|----------------------|
| `crates/common/src/config.rs` | 720 | infra `config` (no TypeScript or verified crate equivalent) |
| `crates/common/src/retry.rs` | 421 | infra `retry` |
| `crates/common/src/telemetry.rs` | 206 | infra `otel` (no TypeScript equivalent) |
| `crates/infrastructure/src/cache.rs` | 553 | infra `cache` |
| `crates/api-rest/src/middleware/rate_limit.rs` | 182 | infra `rate-limit` |

### Maintained (domain-specific, no replacement planned)

| File | Lines | Reason |
|------|-------|--------|
| `crates/domain/src/errors.rs` | 413 | Domain-specific error types |
| `crates/common/src/crypto.rs` | 290 | Benchmark-specific cryptography |
| `crates/common/src/pagination.rs` | 393 | Domain pagination logic |

---

## Circular Dependency Prevention

- **LLM-Infra** is a foundational layer with no upstream dependencies.
- **LLM-Benchmark-Exchange** consumes Infra but never the reverse.
- The **domain crate** remains isolated from external dependencies.

---

## Open Items

| Item | Blocking |
|------|----------|
| Publish `@llm-dev-ops/infra@0.1.0` to the registry | Adding it to the root manifest |
| TypeScript equivalents for `logging` / `tracing` (Rust `infra-otel`) | Those two modules of the integration |
| Verify the `llm-infra-*` Cargo git dependency names and versions | Rust-side Phase 2B claims |
| Write and test integration code importing `@llm-dev-ops/infra` subpaths | Setting `featureFlags.infraIntegration` to `true` |
