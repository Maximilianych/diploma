-- 002_ml_tables.sql

ALTER TABLE tasks ADD COLUMN completed_at DATETIME;

UPDATE tasks 
SET completed_at = updated_at 
WHERE status = 'done' AND completed_at IS NULL;

ALTER TABLE tasks RENAME COLUMN actual_hours TO actual_spent_seconds;
ALTER TABLE tasks RENAME COLUMN predicted_hours TO predicted_seconds;

-- Source-корпус (read-only, заполняется вручную)
CREATE TABLE ml_source_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_project_name TEXT NOT NULL,
    summary TEXT NOT NULL,
    description TEXT,
    actual_spent_seconds INTEGER NOT NULL,
    completed_at DATETIME,
    dataset_version TEXT NOT NULL DEFAULT 'v1'
);

CREATE INDEX idx_ml_source_tasks_version ON ml_source_tasks(dataset_version);


-- Запуски обучения
CREATE TABLE ml_training_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mode TEXT NOT NULL,  -- 'production' или 'evaluation'
    target_project_name TEXT,  -- только для evaluation mode
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at DATETIME,
    status TEXT NOT NULL DEFAULT 'running',  -- 'running', 'completed', 'failed'
    n_target_total INTEGER,
    n_target_train INTEGER,
    n_target_val INTEGER,
    n_target_test INTEGER,  -- только для evaluation
    n_source INTEGER,
    config_json TEXT,
    best_candidate_id INTEGER,
    error_message TEXT
);

-- Кандидаты моделей
CREATE TABLE ml_model_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    training_run_id INTEGER NOT NULL REFERENCES ml_training_runs(id) ON DELETE CASCADE,
    model_family TEXT NOT NULL,  -- 'baseline', 'scratch_ridge', 'retrain_ridge'
    alpha REAL,
    target_share REAL,
    weight_summary REAL,
    weight_description REAL,
    weight_char REAL,
    r2_val REAL,
    medae_val REAL,
    mdape_val REAL,
    r2_test REAL,  -- только для evaluation
    medae_test REAL,
    mdape_test REAL,
    artifact_path TEXT,
    is_selected INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_ml_candidates_run ON ml_model_candidates(training_run_id);

-- Активная модель (одна запись, обновляется при смене модели)
CREATE TABLE ml_active_model (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    model_candidate_id INTEGER NOT NULL REFERENCES ml_model_candidates(id),
    activated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    previous_candidate_id INTEGER
);