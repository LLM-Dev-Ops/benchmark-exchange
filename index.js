'use strict';

const crypto = require('crypto');

// =============================================================================
// Cloud Function: benchmark-exchange-agents
// Runtime: Node.js 20 | Entry point: handler
// =============================================================================

const SERVICE_NAME = 'benchmark-exchange-agents';
const AGENTS = ['publish'];

// =============================================================================
// Contract Schemas
// =============================================================================

const PUBLISH_REQUIRED_FIELDS = [
  'benchmark_id',
  'model_provider',
  'model_name',
  'model_version',
  'aggregate_score',
  'metric_scores',
  'methodology',
  'dataset',
  'sample_size',
  'variance',
  'reproduction_count',
];

const METHODOLOGY_REQUIRED_FIELDS = [
  'framework',
  'evaluation_method',
  'scoring_method',
];

const DATASET_REQUIRED_FIELDS = [
  'dataset_id',
  'dataset_version',
  'example_count',
  'split',
];

function validatePublishRequest(body) {
  const errors = [];

  for (const field of PUBLISH_REQUIRED_FIELDS) {
    if (body[field] === undefined || body[field] === null) {
      errors.push({ field, message: `${field} is required` });
    }
  }

  if (body.methodology && typeof body.methodology === 'object') {
    for (const field of METHODOLOGY_REQUIRED_FIELDS) {
      if (!body.methodology[field]) {
        errors.push({ field: `methodology.${field}`, message: `methodology.${field} is required` });
      }
    }
  } else if (body.methodology === undefined) {
    // Already caught above
  }

  if (body.dataset && typeof body.dataset === 'object') {
    for (const field of DATASET_REQUIRED_FIELDS) {
      if (body.dataset[field] === undefined || body.dataset[field] === null) {
        errors.push({ field: `dataset.${field}`, message: `dataset.${field} is required` });
      }
    }
  }

  if (body.aggregate_score !== undefined && typeof body.aggregate_score !== 'number') {
    errors.push({ field: 'aggregate_score', message: 'aggregate_score must be a number' });
  }

  if (body.sample_size !== undefined && (typeof body.sample_size !== 'number' || body.sample_size < 1)) {
    errors.push({ field: 'sample_size', message: 'sample_size must be a positive integer' });
  }

  if (body.variance !== undefined && typeof body.variance !== 'number') {
    errors.push({ field: 'variance', message: 'variance must be a number' });
  }

  if (body.reproduction_count !== undefined && (typeof body.reproduction_count !== 'number' || body.reproduction_count < 1)) {
    errors.push({ field: 'reproduction_count', message: 'reproduction_count must be a positive integer' });
  }

  return errors;
}

// =============================================================================
// CORS
// =============================================================================

const ALLOWED_ORIGINS = (process.env.CORS_ALLOWED_ORIGINS || '*').split(',').map(s => s.trim());
const ALLOWED_METHODS = 'GET, POST, PUT, OPTIONS';
const ALLOWED_HEADERS = 'Content-Type, Authorization, X-Correlation-Id, X-Request-Id';

function setCorsHeaders(req, res) {
  const origin = req.headers['origin'];
  if (ALLOWED_ORIGINS.includes('*')) {
    res.set('Access-Control-Allow-Origin', '*');
  } else if (origin && ALLOWED_ORIGINS.includes(origin)) {
    res.set('Access-Control-Allow-Origin', origin);
    res.set('Vary', 'Origin');
  }
  res.set('Access-Control-Allow-Methods', ALLOWED_METHODS);
  res.set('Access-Control-Allow-Headers', ALLOWED_HEADERS);
  res.set('Access-Control-Max-Age', '86400');
}

// =============================================================================
// Execution Metadata Builder
// =============================================================================

function buildExecutionMetadata(req) {
  return {
    trace_id: req.headers['x-correlation-id'] || crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    service: SERVICE_NAME,
    execution_id: crypto.randomUUID(),
  };
}

function buildResponse(executionMetadata, data, layers) {
  return {
    ...data,
    execution_metadata: executionMetadata,
    layers_executed: layers,
  };
}

// =============================================================================
// Agent Route: POST /v1/benchmark-exchange/publish
// =============================================================================

function handlePublish(req, executionMetadata) {
  const start = Date.now();
  const layers = [
    { layer: 'AGENT_ROUTING', status: 'completed' },
  ];

  const body = req.body || {};
  const validationErrors = validatePublishRequest(body);

  if (validationErrors.length > 0) {
    layers.push({
      layer: 'BENCHMARK_EXCHANGE_PUBLISH',
      status: 'validation_failed',
      duration_ms: Date.now() - start,
    });
    return {
      statusCode: 400,
      body: buildResponse(executionMetadata, {
        success: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Request validation failed',
          details: validationErrors,
        },
      }, layers),
    };
  }

  const publicationId = crypto.randomUUID();
  const now = new Date().toISOString();

  const publication = {
    id: publicationId,
    benchmark_id: body.benchmark_id,
    submission_id: body.submission_id || null,
    status: 'draft',
    version: '1.0.0',
    model_provider: body.model_provider,
    model_name: body.model_name,
    model_version: body.model_version,
    aggregate_score: body.aggregate_score,
    normalized_score: body.aggregate_score,
    confidence_level: classifyConfidence(body),
    reproducibility_score: computeReproducibility(body),
    published_by: req.headers['x-user-id'] || 'anonymous',
    organization_id: req.headers['x-organization-id'] || null,
    tags: body.tags || [],
    is_latest: true,
    created_at: now,
    updated_at: now,
    published_at: null,
  };

  const decisionEvent = {
    agent_id: 'benchmark-publication-agent',
    agent_version: '1.0.0',
    agent_identity: {
      source_agent: 'benchmark-publication-agent',
      domain: 'benchmark',
      phase: 'phase7',
      layer: 'layer2',
      agent_version: '1.0.0',
    },
    decision_type: 'benchmark_publish',
    inputs_hash: crypto.createHash('sha256').update(JSON.stringify(body)).digest('hex'),
    outputs: {
      publication_id: publicationId,
      status: 'success',
    },
    confidence: {
      reproducibility_score: publication.reproducibility_score,
      sample_size: body.sample_size,
      variance: body.variance,
      reproduction_count: body.reproduction_count,
    },
    execution_ref: executionMetadata.trace_id,
    timestamp: now,
  };

  layers.push({
    layer: 'BENCHMARK_EXCHANGE_PUBLISH',
    status: 'completed',
    duration_ms: Date.now() - start,
  });

  return {
    statusCode: 201,
    body: buildResponse(executionMetadata, {
      success: true,
      data: {
        publication,
        decision_event: decisionEvent,
      },
    }, layers),
  };
}

function classifyConfidence(body) {
  const score = computeReproducibility(body);
  if (score >= 0.95) return 'very_high';
  if (score >= 0.85) return 'high';
  if (score >= 0.70) return 'medium';
  if (score >= 0.50) return 'low';
  return 'very_low';
}

function computeReproducibility(body) {
  const sampleWeight = Math.min(body.sample_size / 1000, 1.0) * 0.3;
  const varianceWeight = Math.max(1.0 - body.variance, 0.0) * 0.4;
  const reproWeight = Math.min(body.reproduction_count / 5, 1.0) * 0.3;
  return Math.min(sampleWeight + varianceWeight + reproWeight, 1.0);
}

// =============================================================================
// Health Endpoint
// =============================================================================

function handleHealth(executionMetadata) {
  const layers = [
    { layer: 'AGENT_ROUTING', status: 'completed' },
  ];

  return {
    statusCode: 200,
    body: buildResponse(executionMetadata, {
      status: 'healthy',
      service: SERVICE_NAME,
      agents: AGENTS,
      version: '1.0.0',
      agent_identity: {
        source_agent: 'benchmark-publication-agent',
        domain: 'benchmark',
        phase: 'phase7',
        layer: 'layer2',
        agent_version: '1.0.0',
      },
    }, layers),
  };
}

// =============================================================================
// Cloud Function Entry Point
// =============================================================================

/**
 * HTTP Cloud Function entry point.
 *
 * @param {import('express').Request} req
 * @param {import('express').Response} res
 */
function handler(req, res) {
  setCorsHeaders(req, res);

  if (req.method === 'OPTIONS') {
    res.status(204).send('');
    return;
  }

  const executionMetadata = buildExecutionMetadata(req);
  const path = req.path.replace(/\/+$/, '') || '/';

  let result;

  switch (true) {
    case path === '/health' && req.method === 'GET':
      result = handleHealth(executionMetadata);
      break;

    case path === '/v1/benchmark-exchange/publish' && req.method === 'POST':
      result = handlePublish(req, executionMetadata);
      break;

    default:
      result = {
        statusCode: 404,
        body: buildResponse(executionMetadata, {
          success: false,
          error: {
            code: 'NOT_FOUND',
            message: `Route ${req.method} ${path} not found`,
            available_routes: [
              'GET  /health',
              'POST /v1/benchmark-exchange/publish',
            ],
          },
        }, [
          { layer: 'AGENT_ROUTING', status: 'not_found' },
        ]),
      };
      break;
  }

  res.status(result.statusCode).json(result.body);
}

module.exports = { handler };
