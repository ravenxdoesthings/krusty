CREATE TABLE analytics_killmails (
    killmail_id BIGINT NOT NULL,
    killmail_hash TEXT NOT NULL,

    fitted_value DOUBLE PRECISION DEFAULT 0,
    destroyed_value DOUBLE PRECISION DEFAULT 0,
    dropped_value DOUBLE PRECISION DEFAULT 0,
    total_value DOUBLE PRECISION DEFAULT 0,
    attacker_count INT DEFAULT 0,

    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),

    PRIMARY KEY (killmail_id, killmail_hash)
)