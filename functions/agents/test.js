'use strict';

const { handler } = require('./index');
const assert = require('assert');

let passed = 0;
let failed = 0;

function mockReq(method, path, body, headers) {
  return {
    method,
    path,
    body: body || {},
    headers: headers || {},
  };
}

function mockRes() {
  const res = {
    _status: null,
    _body: null,
    _headers: {},
    status(code) { res._status = code; return res; },
    json(body) { res._body = body; return res; },
    send(body) { res._body = body; return res; },
    set(key, value) { res._headers[key] = value; return res; },
  };
  return res;
}

function test(name, fn) {
  try {
    fn();
    passed++;
    console.log(`  PASS  ${name}`);
  } catch (err) {
    failed++;
    console.log(`  FAIL  ${name}`);
    console.log(`        ${err.message}`);
  }
}

console.log('benchmark-exchange-agents Cloud Function Tests\n');

// ---------------------------------------------------------------------------
// Health endpoint
// ---------------------------------------------------------------------------

test('GET /health returns 200 with agent list', () => {
  const req = mockReq('GET', '/health');
  const res = mockRes();
  handler(req, res);
  assert.strictEqual(res._status, 200);
  assert.strictEqual(res._body.status, 'healthy');
  assert.deepStrictEqual(res._body.agents, ['publish']);
  assert.strictEqual(res._body.service, 'benchmark-exchange-agents');
});

test('GET /health includes execution_metadata', () => {
  const req = mockReq('GET', '/health');
  const res = mockRes();
  handler(req, res);
  assert.ok(res._body.execution_metadata);
  assert.ok(res._body.execution_metadata.trace_id);
  assert.ok(res._body.execution_metadata.timestamp);
  assert.strictEqual(res._body.execution_metadata.service, 'benchmark-exchange-agents');
  assert.ok(res._body.execution_metadata.execution_id);
});

test('GET /health includes layers_executed', () => {
  const req = mockReq('GET', '/health');
  const res = mockRes();
  handler(req, res);
  assert.ok(Array.isArray(res._body.layers_executed));
  assert.strictEqual(res._body.layers_executed[0].layer, 'AGENT_ROUTING');
  assert.strictEqual(res._body.layers_executed[0].status, 'completed');
});

test('GET /health includes agent_identity', () => {
  const req = mockReq('GET', '/health');
  const res = mockRes();
  handler(req, res);
  const id = res._body.agent_identity;
  assert.strictEqual(id.source_agent, 'benchmark-publication-agent');
  assert.strictEqual(id.domain, 'benchmark');
  assert.strictEqual(id.phase, 'phase7');
  assert.strictEqual(id.layer, 'layer2');
});

// ---------------------------------------------------------------------------
// CORS
// ---------------------------------------------------------------------------

test('OPTIONS returns 204 with CORS headers', () => {
  const req = mockReq('OPTIONS', '/v1/benchmark-exchange/publish', null, { origin: 'https://example.com' });
  const res = mockRes();
  handler(req, res);
  assert.strictEqual(res._status, 204);
  assert.ok(res._headers['Access-Control-Allow-Methods']);
  assert.ok(res._headers['Access-Control-Allow-Headers']);
});

test('CORS headers set on all responses', () => {
  const req = mockReq('GET', '/health', null, { origin: 'https://example.com' });
  const res = mockRes();
  handler(req, res);
  assert.ok(res._headers['Access-Control-Allow-Origin']);
});

// ---------------------------------------------------------------------------
// X-Correlation-Id passthrough
// ---------------------------------------------------------------------------

test('execution_metadata.trace_id uses x-correlation-id when provided', () => {
  const correlationId = 'test-trace-abc-123';
  const req = mockReq('GET', '/health', null, { 'x-correlation-id': correlationId });
  const res = mockRes();
  handler(req, res);
  assert.strictEqual(res._body.execution_metadata.trace_id, correlationId);
});

// ---------------------------------------------------------------------------
// POST /v1/benchmark-exchange/publish — validation
// ---------------------------------------------------------------------------

test('POST /v1/benchmark-exchange/publish returns 400 on empty body', () => {
  const req = mockReq('POST', '/v1/benchmark-exchange/publish', {});
  const res = mockRes();
  handler(req, res);
  assert.strictEqual(res._status, 400);
  assert.strictEqual(res._body.success, false);
  assert.strictEqual(res._body.error.code, 'VALIDATION_ERROR');
  assert.ok(res._body.error.details.length > 0);
});

test('POST /v1/benchmark-exchange/publish validates methodology fields', () => {
  const req = mockReq('POST', '/v1/benchmark-exchange/publish', {
    benchmark_id: 'bench-1',
    model_provider: 'test',
    model_name: 'test-model',
    model_version: '1.0',
    aggregate_score: 0.85,
    metric_scores: {},
    methodology: { framework: 'lm-eval' },
    dataset: { dataset_id: 'ds-1', dataset_version: '1.0', example_count: 100, split: 'test' },
    sample_size: 100,
    variance: 0.05,
    reproduction_count: 3,
  });
  const res = mockRes();
  handler(req, res);
  assert.strictEqual(res._status, 400);
  const fields = res._body.error.details.map(d => d.field);
  assert.ok(fields.includes('methodology.evaluation_method'));
  assert.ok(fields.includes('methodology.scoring_method'));
});

// ---------------------------------------------------------------------------
// POST /v1/benchmark-exchange/publish — success
// ---------------------------------------------------------------------------

test('POST /v1/benchmark-exchange/publish returns 201 on valid request', () => {
  const req = mockReq('POST', '/v1/benchmark-exchange/publish', {
    benchmark_id: 'bench-1',
    submission_id: 'sub-1',
    model_provider: 'anthropic',
    model_name: 'claude-3',
    model_version: '3.5',
    aggregate_score: 0.92,
    metric_scores: { accuracy: { value: 0.92 } },
    methodology: {
      framework: 'lm-eval',
      evaluation_method: 'zero-shot',
      scoring_method: 'exact_match',
    },
    dataset: {
      dataset_id: 'mmlu',
      dataset_version: '1.0',
      example_count: 14042,
      split: 'test',
    },
    sample_size: 1000,
    variance: 0.03,
    reproduction_count: 5,
    tags: ['nlp', 'reasoning'],
  });
  const res = mockRes();
  handler(req, res);
  assert.strictEqual(res._status, 201);
  assert.strictEqual(res._body.success, true);
  assert.ok(res._body.data.publication.id);
  assert.strictEqual(res._body.data.publication.status, 'draft');
  assert.strictEqual(res._body.data.publication.model_provider, 'anthropic');
  assert.strictEqual(res._body.data.publication.model_name, 'claude-3');
});

test('Successful publish includes execution_metadata and layers_executed', () => {
  const req = mockReq('POST', '/v1/benchmark-exchange/publish', {
    benchmark_id: 'bench-1',
    model_provider: 'anthropic',
    model_name: 'claude-3',
    model_version: '3.5',
    aggregate_score: 0.92,
    metric_scores: {},
    methodology: {
      framework: 'lm-eval',
      evaluation_method: 'zero-shot',
      scoring_method: 'exact_match',
    },
    dataset: {
      dataset_id: 'mmlu',
      dataset_version: '1.0',
      example_count: 14042,
      split: 'test',
    },
    sample_size: 1000,
    variance: 0.03,
    reproduction_count: 5,
  });
  const res = mockRes();
  handler(req, res);
  assert.ok(res._body.execution_metadata);
  assert.strictEqual(res._body.execution_metadata.service, 'benchmark-exchange-agents');
  assert.ok(res._body.layers_executed.length >= 2);
  assert.strictEqual(res._body.layers_executed[0].layer, 'AGENT_ROUTING');
  assert.strictEqual(res._body.layers_executed[1].layer, 'BENCHMARK_EXCHANGE_PUBLISH');
  assert.strictEqual(res._body.layers_executed[1].status, 'completed');
  assert.ok(typeof res._body.layers_executed[1].duration_ms === 'number');
});

test('Successful publish includes decision_event with agent_identity', () => {
  const req = mockReq('POST', '/v1/benchmark-exchange/publish', {
    benchmark_id: 'bench-1',
    model_provider: 'anthropic',
    model_name: 'claude-3',
    model_version: '3.5',
    aggregate_score: 0.92,
    metric_scores: {},
    methodology: {
      framework: 'lm-eval',
      evaluation_method: 'zero-shot',
      scoring_method: 'exact_match',
    },
    dataset: {
      dataset_id: 'mmlu',
      dataset_version: '1.0',
      example_count: 14042,
      split: 'test',
    },
    sample_size: 1000,
    variance: 0.03,
    reproduction_count: 5,
  });
  const res = mockRes();
  handler(req, res);
  const de = res._body.data.decision_event;
  assert.strictEqual(de.agent_id, 'benchmark-publication-agent');
  assert.strictEqual(de.decision_type, 'benchmark_publish');
  assert.ok(de.inputs_hash);
  assert.strictEqual(de.agent_identity.domain, 'benchmark');
  assert.strictEqual(de.agent_identity.phase, 'phase7');
  assert.strictEqual(de.agent_identity.layer, 'layer2');
});

// ---------------------------------------------------------------------------
// 404 handling
// ---------------------------------------------------------------------------

test('Unknown route returns 404 with available_routes', () => {
  const req = mockReq('GET', '/v1/nonexistent');
  const res = mockRes();
  handler(req, res);
  assert.strictEqual(res._status, 404);
  assert.strictEqual(res._body.error.code, 'NOT_FOUND');
  assert.ok(res._body.error.available_routes.length > 0);
  assert.ok(res._body.execution_metadata);
  assert.ok(res._body.layers_executed);
});

// ---------------------------------------------------------------------------
// Validation failures also include execution_metadata
// ---------------------------------------------------------------------------

test('Validation failure includes execution_metadata and layers', () => {
  const req = mockReq('POST', '/v1/benchmark-exchange/publish', {});
  const res = mockRes();
  handler(req, res);
  assert.ok(res._body.execution_metadata);
  assert.ok(res._body.layers_executed);
  assert.strictEqual(res._body.layers_executed[1].layer, 'BENCHMARK_EXCHANGE_PUBLISH');
  assert.strictEqual(res._body.layers_executed[1].status, 'validation_failed');
});

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

console.log(`\nResults: ${passed} passed, ${failed} failed, ${passed + failed} total`);
process.exit(failed > 0 ? 1 : 0);
