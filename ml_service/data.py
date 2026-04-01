import logging
from datetime import datetime, timedelta

import numpy as np
import pandas as pd
from sqlalchemy import text

logger = logging.getLogger(__name__)


def clean_text(val):
    if val is None or (isinstance(val, float) and np.isnan(val)):
        return ""
    return str(val).strip()


def load_target_tasks(engine, config) -> pd.DataFrame:
    """Загрузка завершённых задач из таблицы tasks."""
    mode = config["runtime"]["mode"]

    if mode == "evaluation":
        return _load_eval_target(engine, config)

    return _load_production_target(engine, config)


def _load_production_target(engine, config) -> pd.DataFrame:
    query = """
        SELECT id, title AS summary, description,
               actual_spent_seconds, completed_at
        FROM tasks
        WHERE status = 'done'
          AND actual_spent_seconds IS NOT NULL
          AND actual_spent_seconds > 0
          AND completed_at IS NOT NULL
        ORDER BY completed_at
    """
    df = pd.read_sql(query, engine)

    # Фильтрация по окну истории
    history_months = config["history"]["history_months"]
    min_in_window = config["history"]["min_tasks_in_window"]

    if history_months and len(df) > min_in_window:
        cutoff = datetime.utcnow() - timedelta(days=history_months * 30)
        recent = df[df["completed_at"] >= str(cutoff)]
        if len(recent) >= min_in_window:
            df = recent

    df["summary"] = df["summary"].apply(clean_text)
    df["description"] = df["description"].apply(clean_text)
    df["actual_spent_minutes"] = df["actual_spent_seconds"] / 60.0

    logger.info("Loaded %d production target tasks", len(df))
    return df


def _load_eval_target(engine, config) -> pd.DataFrame:
    project = config["runtime"]["eval_target_project"]
    version = config["source_corpus"]["dataset_version"]

    query = text("""
        SELECT id, summary, description,
               actual_spent_seconds, completed_at
        FROM ml_source_tasks
        WHERE source_project_name = :project
          AND dataset_version = :version
          AND actual_spent_seconds > 0
        ORDER BY completed_at, id
    """)
    df = pd.read_sql(query, engine, params={"project": project, "version": version})

    df["summary"] = df["summary"].apply(clean_text)
    df["description"] = df["description"].apply(clean_text)
    df["actual_spent_minutes"] = df["actual_spent_seconds"] / 60.0

    logger.info("Loaded %d eval target tasks (project=%s)", len(df), project)
    return df


def load_source_tasks(engine, config) -> pd.DataFrame:
    """Загрузка source-корпуса из ml_source_tasks."""
    version = config["source_corpus"]["dataset_version"]
    mode = config["runtime"]["mode"]

    params = {"version": version}

    # В evaluation mode исключаем target-проект из source
    if mode == "evaluation":
        project = config["runtime"]["eval_target_project"]
        query = text("""
            SELECT id, source_project_name, summary, description,
                   actual_spent_seconds
            FROM ml_source_tasks
            WHERE dataset_version = :version
              AND source_project_name != :project
              AND actual_spent_seconds > 0
        """)
        params["project"] = project
    else:
        query = text("""
            SELECT id, source_project_name, summary, description,
                   actual_spent_seconds
            FROM ml_source_tasks
            WHERE dataset_version = :version
              AND actual_spent_seconds > 0
        """)

    df = pd.read_sql(query, engine, params=params)

    df["summary"] = df["summary"].apply(clean_text)
    df["description"] = df["description"].apply(clean_text)
    df["actual_spent_minutes"] = df["actual_spent_seconds"] / 60.0

    logger.info("Loaded %d source tasks (%d projects)",
                len(df), df["source_project_name"].nunique())
    return df