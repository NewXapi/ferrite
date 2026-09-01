-- V1 — 核心五域 (来源: todo/spike/new-api-rust-rewrite/07-database-schema.md)
-- 域: Catalog (channels/route_units/groups/model_meta) · Identity (users/tokens)
--     Usage (usage_logs 分区/聚合/健康) · Infra (options/revision/outbox/idempotency/jobs)

-- ============ Catalog ============
CREATE TABLE channels (
    key             UUID PRIMARY KEY,
    logical_version BIGINT NOT NULL,
    origin          TEXT NOT NULL DEFAULT 'center',
    schema_version  INT  NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    name            TEXT NOT NULL,
    provider_type   TEXT NOT NULL,
    base_url        TEXT NOT NULL,
    credentials     BYTEA NOT NULL,
    max_concurrency INT  NOT NULL DEFAULT 10,
    status          SMALLINT NOT NULL DEFAULT 1,
    groups          TEXT[] NOT NULL DEFAULT '{}',
    settings        JSONB NOT NULL DEFAULT '{}'
);

CREATE TABLE route_units (
    key             UUID PRIMARY KEY,
    logical_version BIGINT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'center', schema_version INT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    group_id        TEXT NOT NULL,
    public_model    TEXT NOT NULL,
    channel_key     UUID NOT NULL REFERENCES channels(key) ON DELETE CASCADE,
    key_index       INT  NOT NULL DEFAULT 0,
    upstream_model  TEXT NOT NULL,
    priority        INT  NOT NULL DEFAULT 0,
    weight          INT  NOT NULL DEFAULT 10,
    status          SMALLINT NOT NULL DEFAULT 1
);
CREATE INDEX idx_route_units_lookup ON route_units (group_id, public_model) WHERE status = 1;

CREATE TABLE groups (
    key UUID PRIMARY KEY, logical_version BIGINT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'center', schema_version INT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    id TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    rate_multiplier NUMERIC(10,4) NOT NULL DEFAULT 1.0,
    allowed_models TEXT[] NOT NULL DEFAULT '{}'
);

CREATE TABLE model_meta (
    key UUID PRIMARY KEY, logical_version BIGINT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'center', schema_version INT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    public_model TEXT UNIQUE NOT NULL,
    vendor TEXT NOT NULL, description TEXT NOT NULL DEFAULT '',
    visible BOOLEAN NOT NULL DEFAULT true
);

-- ============ Identity ============
CREATE TABLE users (
    key UUID PRIMARY KEY, logical_version BIGINT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'center', schema_version INT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    username TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL DEFAULT '',
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    quota BIGINT NOT NULL DEFAULT 0,
    used_quota BIGINT NOT NULL DEFAULT 0,
    request_count BIGINT NOT NULL DEFAULT 0,
    group_id TEXT NOT NULL DEFAULT 'default',
    role SMALLINT NOT NULL DEFAULT 1,
    status SMALLINT NOT NULL DEFAULT 1
);

CREATE TABLE tokens (
    key UUID PRIMARY KEY, logical_version BIGINT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'center', schema_version INT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    user_key UUID NOT NULL REFERENCES users(key) ON DELETE CASCADE,
    name TEXT NOT NULL,
    key_hash TEXT UNIQUE NOT NULL,
    key_preview TEXT NOT NULL DEFAULT '',
    group_id TEXT,
    quota BIGINT NOT NULL DEFAULT 0,
    unlimited_quota BOOLEAN NOT NULL DEFAULT false,
    used_quota BIGINT NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    status SMALLINT NOT NULL DEFAULT 1
);
CREATE INDEX idx_tokens_user ON tokens (user_key);

-- ============ Usage (按月分区) ============
CREATE TABLE usage_logs (
    mutation_id UUID PRIMARY KEY,
    token_key UUID NOT NULL, user_key UUID NOT NULL,
    channel_key UUID NOT NULL, route_unit_key UUID,
    public_model TEXT NOT NULL, upstream_model TEXT NOT NULL,
    prompt_tokens BIGINT NOT NULL DEFAULT 0,
    completion_tokens BIGINT NOT NULL DEFAULT 0,
    cached_tokens BIGINT NOT NULL DEFAULT 0,
    first_token_ms INT NOT NULL DEFAULT 0, duration_ms INT NOT NULL DEFAULT 0,
    cost BIGINT NOT NULL DEFAULT 0,
    status_code INT NOT NULL DEFAULT 200,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
) PARTITION BY RANGE (created_at);
-- TODO(#412): 分区自动创建 (pg_partman 或月初定时任务, ops::jobs 挂载)。

CREATE TABLE usage_hourly (
    bucket_start TIMESTAMPTZ NOT NULL,
    user_key UUID NOT NULL, model TEXT NOT NULL,
    group_id TEXT NOT NULL, channel_key UUID NOT NULL,
    requests BIGINT NOT NULL DEFAULT 0,
    prompt_tokens BIGINT NOT NULL DEFAULT 0, completion_tokens BIGINT NOT NULL DEFAULT 0,
    cost BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (bucket_start, user_key, model, channel_key)
);

CREATE TABLE model_rankings (
    period TEXT NOT NULL, model TEXT NOT NULL,
    tokens BIGINT NOT NULL DEFAULT 0, cost BIGINT NOT NULL DEFAULT 0,
    requests BIGINT NOT NULL DEFAULT 0,
    prev_tokens BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (period, model)
);

CREATE TABLE perf_metrics (
    bucket_start TIMESTAMPTZ NOT NULL,
    model TEXT NOT NULL, group_id TEXT NOT NULL,
    requests BIGINT NOT NULL DEFAULT 0, success BIGINT NOT NULL DEFAULT 0,
    ttft_ms_sum BIGINT NOT NULL DEFAULT 0, duration_ms_sum BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (bucket_start, model, group_id)
);

CREATE TABLE health_observations (
    mutation_id UUID PRIMARY KEY,
    channel_key UUID NOT NULL, model TEXT NOT NULL,
    outcome TEXT NOT NULL,
    latency_ms INT NOT NULL DEFAULT 0,
    observed_at TIMESTAMPTZ NOT NULL
);

-- ============ Infra ============
CREATE TABLE options (key TEXT PRIMARY KEY, value JSONB NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT now());

CREATE TABLE config_revision (id BIGINT PRIMARY KEY, watermark BIGINT NOT NULL);
INSERT INTO config_revision (id, watermark) VALUES (1, 0);

CREATE TABLE revision_outbox (
    watermark BIGINT PRIMARY KEY, published BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE idempotency_records (
    scope TEXT NOT NULL, key_hash TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    state TEXT NOT NULL,
    response JSONB, lease_expires TIMESTAMPTZ,
    PRIMARY KEY (scope, key_hash)
);

CREATE TABLE system_jobs (
    key UUID PRIMARY KEY,
    job_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    state TEXT NOT NULL DEFAULT 'pending',
    leased_by TEXT, lease_expires TIMESTAMPTZ,
    result JSONB, attempts INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_system_jobs_claim ON system_jobs (state, lease_expires);
