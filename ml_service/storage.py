import json
import os
import logging

import joblib
from sqlalchemy import create_engine, text

logger = logging.getLogger(__name__)


def get_engine(config):
    url = config["database"]["url"]
    if url.startswith("sqlite"):
        return create_engine(url, connect_args={"timeout": 30})
    return create_engine(url)


# ============ Training Runs ============

def create_training_run(engine, mode, target_project_name, config_dict):
    with engine.begin() as conn:
        result = conn.execute(
            text("""
                INSERT INTO ml_training_runs (mode, target_project_name, config_json)
                VALUES (:mode, :target, :config)
            """),
            {
                "mode": mode,
                "target": target_project_name,
                "config": json.dumps(config_dict, default=str),
            },
        )
        return result.lastrowid


def finish_training_run(
    engine, run_id, status,
    n_target_total=None, n_target_train=None, n_target_val=None,
    n_target_test=None, n_source=None,
    best_candidate_id=None, error_message=None,
):
    with engine.begin() as conn:
        conn.execute(
            text("""
                UPDATE ml_training_runs
                SET finished_at = CURRENT_TIMESTAMP,
                    status = :status,
                    n_target_total = :n_total,
                    n_target_train = :n_train,
                    n_target_val = :n_val,
                    n_target_test = :n_test,
                    n_source = :n_source,
                    best_candidate_id = :best_id,
                    error_message = :error
                WHERE id = :run_id
            """),
            {
                "status": status,
                "n_total": n_target_total,
                "n_train": n_target_train,
                "n_val": n_target_val,
                "n_test": n_target_test,
                "n_source": n_source,
                "best_id": best_candidate_id,
                "error": error_message,
                "run_id": run_id,
            },
        )


# ============ Model Candidates ============

def save_candidate(
    engine, run_id, model_family,
    alpha, target_share, ws, wd, wc,
    metrics_val, metrics_test, artifact_path, is_selected,
):
    with engine.begin() as conn:
        result = conn.execute(
            text("""
                INSERT INTO ml_model_candidates
                (training_run_id, model_family, alpha, target_share,
                 weight_summary, weight_description, weight_char,
                 r2_val, medae_val, mdape_val,
                 r2_test, medae_test, mdape_test,
                 artifact_path, is_selected)
                VALUES (:run_id, :family, :alpha, :ts,
                        :ws, :wd, :wc,
                        :r2v, :medaev, :mdapev,
                        :r2t, :medaet, :mdapet,
                        :path, :selected)
            """),
            {
                "run_id": run_id,
                "family": model_family,
                "alpha": alpha,
                "ts": target_share,
                "ws": ws, "wd": wd, "wc": wc,
                "r2v": metrics_val.get("r2"),
                "medaev": metrics_val.get("medae"),
                "mdapev": metrics_val.get("mdape"),
                "r2t": metrics_test.get("r2") if metrics_test else None,
                "medaet": metrics_test.get("medae") if metrics_test else None,
                "mdapet": metrics_test.get("mdape") if metrics_test else None,
                "path": artifact_path,
                "selected": 1 if is_selected else 0,
            },
        )
        return result.lastrowid


# ============ Active Model ============

def get_active_model_info(engine):
    with engine.connect() as conn:
        row = conn.execute(
            text("""
                SELECT am.model_candidate_id, mc.artifact_path, mc.model_family,
                       mc.r2_val, mc.medae_val, mc.mdape_val, am.activated_at
                FROM ml_active_model am
                JOIN ml_model_candidates mc ON am.model_candidate_id = mc.id
                WHERE am.id = 1
            """)
        ).fetchone()
        if row is None:
            return None
        return dict(row._mapping)


def activate_model(engine, candidate_id):
    with engine.begin() as conn:
        current = conn.execute(
            text("SELECT model_candidate_id FROM ml_active_model WHERE id = 1")
        ).fetchone()

        previous_id = current[0] if current else None

        conn.execute(
            text("""
                INSERT INTO ml_active_model
                    (id, model_candidate_id, activated_at, previous_candidate_id)
                VALUES (1, :cid, CURRENT_TIMESTAMP, :prev)
                ON CONFLICT(id) DO UPDATE SET
                    model_candidate_id = :cid,
                    activated_at = CURRENT_TIMESTAMP,
                    previous_candidate_id = :prev
            """),
            {"cid": candidate_id, "prev": previous_id},
        )
    logger.info("Activated model candidate %d (previous: %s)", candidate_id, previous_id)


# ============ Artifacts ============

def save_artifact(artifact, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    joblib.dump(artifact, path)
    logger.info("Saved artifact to %s", path)


def load_artifact(path):
    if not os.path.exists(path):
        logger.warning("Artifact not found: %s", path)
        return None
    return joblib.load(path)