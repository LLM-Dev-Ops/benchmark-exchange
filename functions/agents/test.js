'use strict';

process.env.CORS_ALLOWED_ORIGINS = 'https://example.com';

const { handler } = require(process.env.BENCHMARK_AGENT_ENTRY || './index');
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

function validPublishBody() {
  return {
    benchmark_id: 'bench-1',
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
  };
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

test('CORS headers set for an explicitly allowed origin', () => {
  const req = mockReq('GET', '/health', null, { origin: 'https://example.com' });
  const res = mockRes();
  handler(req, res);
  assert.strictEqual(res._headers['Access-Control-Allow-Origin'], 'https://example.com');
  assert.strictEqual(res._headers.Vary, 'Origin');
});

test('CORS does not reflect an unconfigured origin', () => {
  const req = mockReq('GET', '/health', null, { origin: 'https://evil.example' });
  const res = mockRes();
  handler(req, res);
  assert.strictEqual(res._headers['Access-Control-Allow-Origin'], undefined);
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

test('invalid correlation IDs are replaced', () => {
  for (const correlationId of ['bad\nheader', 'x'.repeat(129)]) {
    const req = mockReq('GET', '/health', null, { 'x-correlation-id': correlationId });
    const res = mockRes();
    handler(req, res);
    assert.notStrictEqual(res._body.execution_metadata.trace_id, correlationId);
    assert.match(res._body.execution_metadata.trace_id, /^[0-9a-f-]{36}$/);
  }
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

test('POST /v1/benchmark-exchange/publish rejects non-object bodies and nested contracts', () => {
  const invalidBodies = [
    [],
    { ...validPublishBody(), methodology: [] },
    { ...validPublishBody(), dataset: 'mmlu' },
    { ...validPublishBody(), metric_scores: [] },
    { ...validPublishBody(), dataset: { ...validPublishBody().dataset, example_count: 1.5 } },
  ];

  for (const body of invalidBodies) {
    const req = mockReq('POST', '/v1/benchmark-exchange/publish', body);
    const res = mockRes();
    handler(req, res);
    assert.strictEqual(res._status, 400);
  }
});

test('POST /v1/benchmark-exchange/publish rejects unsafe numeric values', () => {
  const invalidValues = [
    ['aggregate_score', -0.01],
    ['aggregate_score', 1.01],
    ['aggregate_score', Number.NaN],
    ['sample_size', 1.5],
    ['sample_size', Number.POSITIVE_INFINITY],
    ['variance', -0.01],
    ['variance', Number.NaN],
    ['reproduction_count', 0],
    ['reproduction_count', 1.5],
  ];

  for (const [field, value] of invalidValues) {
    const req = mockReq('POST', '/v1/benchmark-exchange/publish', {
      ...validPublishBody(),
      [field]: value,
    });
    const res = mockRes();
    handler(req, res);
    assert.strictEqual(res._status, 400, `${field}=${value} should be rejected`);
    assert.ok(res._body.error.details.some(detail => detail.field === field));
  }
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
