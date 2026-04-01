import logging
import os

import numpy as np
from scipy.sparse import vstack as sparse_vstack

from config import load_config
from data import load_target_tasks, load_source_tasks
from features import FeatureExtractor, combine_blocks
from models import (
    BaselineModel, train_ridge, predict_and_evaluate, compute_metrics,
)
from storage import (
    get_engine, create_training_run, finish_training_run,
    save_candidate, save_artifact, activate_model, get_active_model_info,
)

logger = logging.getLogger(__name__)


# ============ Temporal Split ============

def temporal_split(df, config):
    """Разбивает по времени на train/val (production) или train/val/test (evaluation)."""
    mode = config["runtime"]["mode"]
    n = len(df)

    if mode == "evaluation":
        test_ratio = config["training"]["test_ratio"]
        val_ratio = config["training"]["validation_ratio"]
        train_ratio = 1.0 - val_ratio - test_ratio

        n_train = int(n * train_ratio)
        n_val = int(n * val_ratio)

        train_df = df.iloc[:n_train]
        val_df = df.iloc[n_train:n_train + n_val]
        test_df = df.iloc[n_train + n_val:]

        logger.info("Eval split: train=%d, val=%d, test=%d", len(train_df), len(val_df), len(test_df))
        return train_df, val_df, test_df
    else:
        val_ratio = config["training"]["validation_ratio"]
        n_train = int(n * (1.0 - val_ratio))

        train_df = df.iloc[:n_train]
        val_df = df.iloc[n_train:]

        logger.info("Production split: train=%d, val=%d", len(train_df), len(val_df))
        return train_df, val_df, None


# ============ Staged Search: Scratch ============

def staged_search_scratch(train_df, val_df, config):
    """Поэтапный подбор гиперпараметров для Scratch Ridge."""
    scratch_cfg = config["scratch"]

    summaries_train = train_df["summary"].tolist()
    descriptions_train = train_df["description"].tolist()
    summaries_val = val_df["summary"].tolist()
    descriptions_val = val_df["description"].tolist()
    y_train = np.log1p(train_df["actual_spent_minutes"].values)
    y_val = np.log1p(val_df["actual_spent_minutes"].values)

    # Fit vectorizers на target_train
    extractor = FeatureExtractor(config)
    X_s_train, X_d_train, X_c_train = extractor.fit_transform_blocks(
        summaries_train, descriptions_train
    )
    X_s_val, X_d_val, X_c_val = extractor.transform_blocks(
        summaries_val, descriptions_val
    )

    # Stage 1: веса (1,1,1), перебор alpha
    default_weights = [1.0, 1.0, 1.0]
    X_train = combine_blocks(X_s_train, X_d_train, X_c_train, default_weights)
    X_val = combine_blocks(X_s_val, X_d_val, X_c_val, default_weights)

    best_alpha = None
    best_metrics_s1 = None

    for alpha in scratch_cfg["alpha_grid"]:
        model = train_ridge(X_train, y_train, alpha)
        metrics = predict_and_evaluate(model, X_val, y_val)
        logger.info("Scratch stage1: alpha=%.1f -> R²=%.4f MdAPE=%.1f",
                     alpha, metrics["r2"], metrics["mdape"])
        if _is_better(metrics, best_metrics_s1, config):
            best_alpha = alpha
            best_metrics_s1 = metrics

    # Stage 2: фиксируем alpha, перебор block weights
    best_weights = default_weights
    best_metrics = best_metrics_s1
    best_model = None

    for weights in scratch_cfg["block_weight_grid"]:
        X_train = combine_blocks(X_s_train, X_d_train, X_c_train, weights)
        X_val = combine_blocks(X_s_val, X_d_val, X_c_val, weights)

        model = train_ridge(X_train, y_train, best_alpha)
        metrics = predict_and_evaluate(model, X_val, y_val)
        logger.info("Scratch stage2: weights=%s -> R²=%.4f MdAPE=%.1f",
                     weights, metrics["r2"], metrics["mdape"])
        if _is_better(metrics, best_metrics, config):
            best_weights = weights
            best_metrics = metrics
            best_model = model

    # Если stage2 не нашёл лучше, обучаем с лучшим alpha и default weights
    if best_model is None:
        X_train = combine_blocks(X_s_train, X_d_train, X_c_train, default_weights)
        best_model = train_ridge(X_train, y_train, best_alpha)
        best_weights = default_weights

    return {
        "family": "scratch_ridge",
        "alpha": best_alpha,
        "target_share": None,
        "weights": best_weights,
        "metrics_val": best_metrics,
        "model": best_model,
        "extractor": extractor,
    }


# ============ Staged Search: Retrain ============

def staged_search_retrain(source_df, train_df, val_df, config):
    """Поэтапный подбор гиперпараметров для Retrain Ridge."""
    retrain_cfg = config["retrain"]

    # Тексты
    all_summaries = source_df["summary"].tolist() + train_df["summary"].tolist()
    all_descriptions = source_df["description"].tolist() + train_df["description"].tolist()
    summaries_val = val_df["summary"].tolist()
    descriptions_val = val_df["description"].tolist()

    y_source = np.log1p(source_df["actual_spent_minutes"].values)
    y_train = np.log1p(train_df["actual_spent_minutes"].values)
    y_val = np.log1p(val_df["actual_spent_minutes"].values)
    y_combined = np.concatenate([y_source, y_train])

    n_source = len(source_df)
    n_target = len(train_df)

    # Fit vectorizers на source + target_train
    extractor = FeatureExtractor(config)
    extractor.fit(all_summaries, all_descriptions)

    # Transform раздельно для возможности vstack
    X_s_source, X_d_source, X_c_source = extractor.transform_blocks(
        source_df["summary"].tolist(), source_df["description"].tolist()
    )
    X_s_target, X_d_target, X_c_target = extractor.transform_blocks(
        train_df["summary"].tolist(), train_df["description"].tolist()
    )
    X_s_val, X_d_val, X_c_val = extractor.transform_blocks(
        summaries_val, descriptions_val
    )

    # Объединяем source + target блоки
    X_s_combined = sparse_vstack([X_s_source, X_s_target])
    X_d_combined = sparse_vstack([X_d_source, X_d_target])
    X_c_combined = sparse_vstack([X_c_source, X_c_target])

    # Stage 1: веса (1,1,1), перебор alpha × target_share
    default_weights = [1.0, 1.0, 1.0]
    X_train = combine_blocks(X_s_combined, X_d_combined, X_c_combined, default_weights)
    X_val = combine_blocks(X_s_val, X_d_val, X_c_val, default_weights)

    best_alpha = None
    best_ts = None
    best_metrics_s1 = None

    for alpha in retrain_cfg["alpha_grid"]:
        for ts in retrain_cfg["target_share_grid"]:
            w_target = ts * n_source / ((1 - ts) * n_target) if n_target > 0 else 1.0
            sample_weight = np.concatenate([
                np.ones(n_source),
                np.full(n_target, w_target),
            ])

            model = train_ridge(X_train, y_combined, alpha, sample_weight)
            metrics = predict_and_evaluate(model, X_val, y_val)
            logger.info("Retrain stage1: alpha=%.1f ts=%.2f -> R²=%.4f MdAPE=%.1f",
                         alpha, ts, metrics["r2"], metrics["mdape"])
            if _is_better(metrics, best_metrics_s1, config):
                best_alpha = alpha
                best_ts = ts
                best_metrics_s1 = metrics

    # Stage 2: фиксируем alpha и target_share, перебор block weights
    w_target = best_ts * n_source / ((1 - best_ts) * n_target) if n_target > 0 else 1.0
    sample_weight = np.concatenate([
        np.ones(n_source),
        np.full(n_target, w_target),
    ])

    best_weights = default_weights
    best_metrics = best_metrics_s1
    best_model = None

    for weights in retrain_cfg["block_weight_grid"]:
        X_train = combine_blocks(X_s_combined, X_d_combined, X_c_combined, weights)
        X_val = combine_blocks(X_s_val, X_d_val, X_c_val, weights)

        model = train_ridge(X_train, y_combined, best_alpha, sample_weight)
        metrics = predict_and_evaluate(model, X_val, y_val)
        logger.info("Retrain stage2: weights=%s -> R²=%.4f MdAPE=%.1f",
                     weights, metrics["r2"], metrics["mdape"])
        if _is_better(metrics, best_metrics, config):
            best_weights = weights
            best_metrics = metrics
            best_model = model

    if best_model is None:
        X_train = combine_blocks(X_s_combined, X_d_combined, X_c_combined, default_weights)
        best_model = train_ridge(X_train, y_combined, best_alpha, sample_weight)
        best_weights = default_weights

    return {
        "family": "retrain_ridge",
        "alpha": best_alpha,
        "target_share": best_ts,
        "weights": best_weights,
        "metrics_val": best_metrics,
        "model": best_model,
        "extractor": extractor,
    }


# ============ Model Selection ============

def _is_better(new_metrics, current_best, config):
    """Проверяет, лучше ли new_metrics чем current_best."""
    if current_best is None:
        return True

    sel = config["selection"]
    primary = sel["primary_metric"]

    if sel["require_non_negative_r2"] and new_metrics["r2"] < 0:
        return False

    if primary == "mdape":
        return new_metrics["mdape"] < current_best["mdape"]
    elif primary == "medae":
        return new_metrics["medae"] < current_best["medae"]
    else:  # r2
        return new_metrics["r2"] > current_best["r2"]


def select_best(candidates, config):
    """Выбирает лучшего кандидата из списка."""
    sel = config["selection"]
    primary = sel["primary_metric"]

    # Фильтруем кандидатов с R² < 0 если требуется
    filtered = candidates
    if sel["require_non_negative_r2"]:
        non_neg = [c for c in candidates if c["metrics_val"]["r2"] >= 0]
        if non_neg:
            filtered = non_neg

    if primary == "mdape":
        return min(filtered, key=lambda c: c["metrics_val"]["mdape"])
    elif primary == "medae":
        return min(filtered, key=lambda c: c["metrics_val"]["medae"])
    else:
        return max(filtered, key=lambda c: c["metrics_val"]["r2"])


# ============ Refit ============

def refit_model(candidate, target_full_df, source_df, config):
    """Переобучает лучшую модель на всех доступных данных (train+val)."""
    family = candidate["family"]

    if family == "baseline":
        y_all = np.log1p(target_full_df["actual_spent_minutes"].values)
        baseline = BaselineModel()
        baseline.fit(y_all)
        candidate["model"] = baseline
        candidate["extractor"] = None
        return candidate

    summaries = target_full_df["summary"].tolist()
    descriptions = target_full_df["description"].tolist()
    y_target = np.log1p(target_full_df["actual_spent_minutes"].values)

    if family == "retrain_ridge" and source_df is not None:
        all_summaries = source_df["summary"].tolist() + summaries
        all_descriptions = source_df["description"].tolist() + descriptions
        y_source = np.log1p(source_df["actual_spent_minutes"].values)
        y_all = np.concatenate([y_source, y_target])

        n_source = len(source_df)
        n_target = len(target_full_df)
        ts = candidate["target_share"]
        w_target = ts * n_source / ((1 - ts) * n_target) if n_target > 0 else 1.0
        sample_weight = np.concatenate([
            np.ones(n_source),
            np.full(n_target, w_target),
        ])
    else:
        all_summaries = summaries
        all_descriptions = descriptions
        y_all = y_target
        sample_weight = None

    extractor = FeatureExtractor(config)
    X_s, X_d, X_c = extractor.fit_transform_blocks(all_summaries, all_descriptions)
    X = combine_blocks(X_s, X_d, X_c, candidate["weights"])

    model = train_ridge(X, y_all, candidate["alpha"], sample_weight)

    candidate["model"] = model
    candidate["extractor"] = extractor
    return candidate


# ============ Evaluate on Test ============

def evaluate_on_test(candidate, test_df, config):
    """Оценка на тестовом наборе (evaluation mode)."""
    if candidate["family"] == "baseline":
        y_test = np.log1p(test_df["actual_spent_minutes"].values)
        y_pred_log = candidate["model"].predict(len(test_df))
        y_pred = np.expm1(y_pred_log)
        y_true = np.expm1(y_test)
        return compute_metrics(y_true, y_pred)

    extractor = candidate["extractor"]
    X_s, X_d, X_c = extractor.transform_blocks(
        test_df["summary"].tolist(), test_df["description"].tolist()
    )
    X = combine_blocks(X_s, X_d, X_c, candidate["weights"])

    y_test = np.log1p(test_df["actual_spent_minutes"].values)
    return predict_and_evaluate(candidate["model"], X, y_test)


# ============ Check Improvement ============

def is_significant_improvement(new_metrics, current_info, config):
    """Проверяет, значительно ли новая модель лучше текущей активной."""
    if current_info is None:
        return True

    sel = config["selection"]
    primary = sel["primary_metric"]

    if primary == "mdape":
        current_val = current_info.get("mdape_val")
        if current_val is None:
            return True
        improvement = current_val - new_metrics["mdape"]
        return improvement >= sel["min_mdape_improvement_pp"]
    else:
        current_val = current_info.get("r2_val")
        if current_val is None:
            return True
        improvement = new_metrics["r2"] - current_val
        return improvement >= sel["min_r2_improvement"]


# ============ Main Pipeline ============

def run_training(config):
    """Основной pipeline обучения."""
    engine = get_engine(config)
    mode = config["runtime"]["mode"]
    target_project = config["runtime"].get("eval_target_project")

    run_id = create_training_run(engine, mode, target_project, config)
    logger.info("Started training run %d (mode=%s)", run_id, mode)

    try:
        # Загрузка данных
        target_df = load_target_tasks(engine, config)
        n_target = len(target_df)

        source_df = None
        n_source = 0
        if config["source_corpus"]["enabled"]:
            source_df = load_source_tasks(engine, config)
            n_source = len(source_df)
            if n_source == 0:
                logger.warning("Source corpus is empty, disabling retrain")
                source_df = None

        # Проверка минимума задач
        min_tasks = config["training"]["min_tasks_for_retrain"]
        if n_target < min_tasks:
            msg = f"Not enough target tasks: {n_target} < {min_tasks}"
            logger.warning(msg)
            finish_training_run(engine, run_id, "failed",
                                n_target_total=n_target, n_source=n_source,
                                error_message=msg)
            return {"status": "failed", "message": msg, "run_id": run_id}

        # Temporal split
        train_df, val_df, test_df = temporal_split(target_df, config)

        candidates = []

        # Baseline
        y_train_log = np.log1p(train_df["actual_spent_minutes"].values)
        y_val_log = np.log1p(val_df["actual_spent_minutes"].values)

        baseline = BaselineModel()
        baseline.fit(y_train_log)
        baseline_pred_log = baseline.predict(len(val_df))
        baseline_metrics = compute_metrics(
            np.expm1(y_val_log), np.expm1(baseline_pred_log)
        )
        logger.info("Baseline: R²=%.4f MdAPE=%.1f",
                     baseline_metrics["r2"], baseline_metrics["mdape"])

        candidates.append({
            "family": "baseline",
            "alpha": None,
            "target_share": None,
            "weights": [1.0, 1.0, 1.0],
            "metrics_val": baseline_metrics,
            "model": baseline,
            "extractor": None,
        })

        # Scratch
        min_scratch = config["training"]["min_tasks_for_scratch"]
        if config["scratch"]["enabled"] and len(train_df) >= min_scratch:
            logger.info("--- Scratch Ridge ---")
            scratch = staged_search_scratch(train_df, val_df, config)
            candidates.append(scratch)
            logger.info("Best Scratch: R²=%.4f MdAPE=%.1f",
                         scratch["metrics_val"]["r2"], scratch["metrics_val"]["mdape"])

        # Retrain
        if (config["retrain"]["enabled"]
                and source_df is not None
                and len(source_df) > 0
                and len(train_df) >= min_tasks):
            logger.info("--- Retrain Ridge ---")
            retrain = staged_search_retrain(source_df, train_df, val_df, config)
            candidates.append(retrain)
            logger.info("Best Retrain: R²=%.4f MdAPE=%.1f",
                         retrain["metrics_val"]["r2"], retrain["metrics_val"]["mdape"])

        # Выбор лучшего
        best = select_best(candidates, config)
        logger.info("Selected: %s (R²=%.4f MdAPE=%.1f)",
                     best["family"], best["metrics_val"]["r2"], best["metrics_val"]["mdape"])

        # Evaluation on test
        metrics_test = None
        if test_df is not None and len(test_df) > 0:
            metrics_test = evaluate_on_test(best, test_df, config)
            logger.info("Test metrics: R²=%.4f MdAPE=%.1f",
                         metrics_test["r2"], metrics_test["mdape"])

        # Refit в production mode
        if mode == "production":
            best = refit_model(best, target_df, source_df, config)

        # Сохранение артефакта
        artifact_dir = config["storage"]["artifact_dir"]
        artifact_path = os.path.join(artifact_dir, f"run_{run_id}", "model.joblib")

        artifact = {
            "model": best["model"],
            "extractor": best["extractor"],
            "weights": best["weights"],
            "family": best["family"],
            "alpha": best["alpha"],
            "target_share": best["target_share"],
        }
        save_artifact(artifact, artifact_path)

        # Сохранение кандидатов в БД
        best_candidate_id = None
        for cand in candidates:
            is_selected = cand is best
            cid = save_candidate(
                engine, run_id,
                model_family=cand["family"],
                alpha=cand.get("alpha"),
                target_share=cand.get("target_share"),
                ws=cand["weights"][0],
                wd=cand["weights"][1],
                wc=cand["weights"][2],
                metrics_val=cand["metrics_val"],
                metrics_test=metrics_test if is_selected else None,
                artifact_path=artifact_path if is_selected else None,
                is_selected=is_selected,
            )
            if is_selected:
                best_candidate_id = cid

        # Активация модели (только production)
        activated = False
        if mode == "production" and best["family"] != "baseline":
            current = get_active_model_info(engine)
            if is_significant_improvement(best["metrics_val"], current, config):
                activate_model(engine, best_candidate_id)
                activated = True
                logger.info("New model activated: candidate %d", best_candidate_id)
            else:
                logger.info("New model not significantly better, keeping current")

        # Финализация
        finish_training_run(
            engine, run_id, "completed",
            n_target_total=n_target,
            n_target_train=len(train_df),
            n_target_val=len(val_df),
            n_target_test=len(test_df) if test_df is not None else None,
            n_source=n_source,
            best_candidate_id=best_candidate_id,
        )

        result = {
            "status": "completed",
            "run_id": run_id,
            "best_model": best["family"],
            "metrics_val": best["metrics_val"],
            "metrics_test": metrics_test,
            "activated": activated,
            "n_target": n_target,
            "n_source": n_source,
        }
        logger.info("Training completed: %s", result)
        return result

    except Exception as e:
        logger.exception("Training failed")
        finish_training_run(engine, run_id, "failed", error_message=str(e))
        raise