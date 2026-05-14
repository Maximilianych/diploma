import math
import numpy as np
import pandas as pd
import pytest
from unittest.mock import MagicMock
from httpx import AsyncClient, ASGITransport

from data import clean_text
from features import FeatureExtractor, combine_blocks
from models import BaselineModel, compute_metrics
from training import temporal_split
from main import app


# ============ clean_text ============

def test_clean_text_normal():
    assert clean_text("Fix login bug") == "Fix login bug"

def test_clean_text_strips_whitespace():
    assert clean_text("  Fix login bug  ") == "Fix login bug"

def test_clean_text_none():
    assert clean_text(None) == ""

def test_clean_text_nan():
    assert clean_text(float("nan")) == ""

def test_clean_text_number():
    assert clean_text(42) == "42"


# ============ compute_metrics ============

def test_compute_metrics_perfect_prediction():
    y_true = np.array([10.0, 20.0, 30.0])
    y_pred = np.array([10.0, 20.0, 30.0])
    metrics = compute_metrics(y_true, y_pred)
    assert metrics["r2"] == pytest.approx(1.0)
    assert metrics["medae"] == pytest.approx(0.0)
    assert metrics["mdape"] == pytest.approx(0.0)

def test_compute_metrics_negative_predictions_clamped():
    y_true = np.array([10.0, 20.0])
    y_pred = np.array([-5.0, 20.0])  # отрицательное предсказание
    metrics = compute_metrics(y_true, y_pred)
    # Не должно падать, отрицательные округляются до нуля
    assert "r2" in metrics
    assert "medae" in metrics
    assert "mdape" in metrics

def test_compute_metrics_all_zero_true():
    # Если все y_true == 0, MdAPE не считается
    y_true = np.array([0.0, 0.0, 0.0])
    y_pred = np.array([1.0, 2.0, 3.0])
    metrics = compute_metrics(y_true, y_pred)
    assert math.isinf(metrics["mdape"])

def test_compute_metrics_reasonable_values():
    y_true = np.array([60.0, 120.0, 180.0, 240.0])
    y_pred = np.array([70.0, 110.0, 200.0, 220.0])
    metrics = compute_metrics(y_true, y_pred)
    assert metrics["r2"] < 1.0
    assert metrics["medae"] > 0.0
    assert metrics["mdape"] > 0.0


# ============ BaselineModel ============

def test_baseline_median():
    y_train_log = np.log1p(np.array([60.0, 120.0, 180.0, 240.0, 300.0]))
    model = BaselineModel()
    model.fit(y_train_log)
    preds = model.predict(3)
    assert len(preds) == 3
    # Все предсказания одинаковы и равны медиане
    assert np.all(preds == preds[0])
    expected_median = float(np.median(y_train_log))
    assert preds[0] == pytest.approx(expected_median)

def test_baseline_single_element():
    y_train_log = np.log1p(np.array([120.0]))
    model = BaselineModel()
    model.fit(y_train_log)
    preds = model.predict(5)
    assert len(preds) == 5
    assert preds[0] == pytest.approx(np.log1p(120.0))


# ============ FeatureExtractor ============

MINIMAL_CONFIG = {
    "features": {
        "summary": {
            "analyzer": "word",
            "ngram_range": [1, 1],
            "max_features": 100,
            "min_df": 1,
            "max_df": 1.0,
            "sublinear_tf": True,
        },
        "description": {
            "analyzer": "word",
            "ngram_range": [1, 1],
            "max_features": 100,
            "min_df": 1,
            "max_df": 1.0,
            "sublinear_tf": True,
        },
        "char": {
            "analyzer": "char_wb",
            "ngram_range": [3, 4],
            "max_features": 100,
            "min_df": 1,
            "max_df": 1.0,
            "sublinear_tf": True,
        },
    }
}

SUMMARIES = [
    "Fix login bug",
    "Add user registration",
    "Update database schema",
    "Write unit tests",
    "Deploy to production",
]

DESCRIPTIONS = [
    "Users cannot login after password reset",
    "Implement email and password registration",
    "Add new columns to users table",
    "Cover auth module with tests",
    "Set up CI/CD pipeline",
]

def test_feature_extractor_fit_transform_shapes():
    extractor = FeatureExtractor(MINIMAL_CONFIG)
    X_s, X_d, X_c = extractor.fit_transform_blocks(SUMMARIES, DESCRIPTIONS)
    n = len(SUMMARIES)
    assert X_s.shape[0] == n
    assert X_d.shape[0] == n
    assert X_c.shape[0] == n

def test_feature_extractor_transform_same_shape():
    extractor = FeatureExtractor(MINIMAL_CONFIG)
    extractor.fit(SUMMARIES, DESCRIPTIONS)
    X_s, X_d, X_c = extractor.transform_blocks(SUMMARIES[:2], DESCRIPTIONS[:2])
    assert X_s.shape[0] == 2
    assert X_d.shape[0] == 2
    assert X_c.shape[0] == 2

def test_combine_blocks_shape():
    extractor = FeatureExtractor(MINIMAL_CONFIG)
    X_s, X_d, X_c = extractor.fit_transform_blocks(SUMMARIES, DESCRIPTIONS)
    weights = [1.0, 2.0, 1.0]
    X = combine_blocks(X_s, X_d, X_c, weights)
    assert X.shape[0] == len(SUMMARIES)
    assert X.shape[1] == X_s.shape[1] + X_d.shape[1] + X_c.shape[1]

def test_combine_blocks_weights_scale():
    extractor = FeatureExtractor(MINIMAL_CONFIG)
    X_s, X_d, X_c = extractor.fit_transform_blocks(SUMMARIES, DESCRIPTIONS)

    X_ones = combine_blocks(X_s, X_d, X_c, [1.0, 1.0, 1.0])
    X_scaled = combine_blocks(X_s, X_d, X_c, [2.0, 1.0, 1.0])

    # Первый блок должен быть в 2 раза больше
    n_s = X_s.shape[1]
    ratio = X_scaled[:, :n_s].sum() / X_ones[:, :n_s].sum()
    assert ratio == pytest.approx(2.0, rel=1e-5)


# ============ temporal_split ============

def make_df(n):
    return pd.DataFrame({
        "summary": [f"task {i}" for i in range(n)],
        "description": ["" for _ in range(n)],
        "actual_spent_minutes": [float(i * 10 + 10) for i in range(n)],
        "completed_at": pd.date_range("2024-01-01", periods=n, freq="D"),
    })

def test_temporal_split_production_proportions():
    config = {
        "runtime": {"mode": "production"},
        "training": {"validation_ratio": 0.2, "test_ratio": 0.2},
    }
    df = make_df(100)
    train, val, test = temporal_split(df, config)

    assert test is None
    assert len(train) + len(val) == 100
    assert len(val) == pytest.approx(20, abs=1)

def test_temporal_split_evaluation_proportions():
    config = {
        "runtime": {"mode": "evaluation"},
        "training": {"validation_ratio": 0.16, "test_ratio": 0.2},
    }
    df = make_df(100)
    train, val, test = temporal_split(df, config)

    assert test is not None
    assert len(train) + len(val) + len(test) == 100
    assert len(test) == pytest.approx(20, abs=1)

def test_temporal_split_order_preserved():
    config = {
        "runtime": {"mode": "production"},
        "training": {"validation_ratio": 0.2, "test_ratio": 0.2},
    }
    df = make_df(50)
    train, val, _ = temporal_split(df, config)

    # Последний элемент train должен быть раньше первого элемента val
    last_train_date = train["completed_at"].iloc[-1]
    first_val_date = val["completed_at"].iloc[0]
    assert last_train_date < first_val_date

def test_temporal_split_small_dataset():
    config = {
        "runtime": {"mode": "production"},
        "training": {"validation_ratio": 0.2, "test_ratio": 0.2},
    }
    df = make_df(10)
    train, val, test = temporal_split(df, config)

    assert test is None
    assert len(train) + len(val) == 10
    assert len(train) > 0
    assert len(val) > 0


# ============ FastAPI endpoints ============

@pytest.fixture
def mock_predictor(monkeypatch):
    """Заменяет глобальный predictor в main.py на mock"""
    import main as main_module
    mock = MagicMock()
    mock.predict.return_value = {
        "predicted_seconds": 7200,
        "model_type": "scratch_ridge",
    }
    monkeypatch.setattr(main_module, "predictor", mock)
    return mock

@pytest.fixture
def no_model_predictor(monkeypatch):
    """Predictor без активной модели"""
    import main as main_module
    mock = MagicMock()
    mock.predict.return_value = None
    monkeypatch.setattr(main_module, "predictor", mock)
    return mock

@pytest.mark.asyncio
async def test_health_endpoint():
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        response = await client.get("/health")
    assert response.status_code == 200
    assert response.json()["status"] == "ok"

@pytest.mark.asyncio
async def test_predict_success(mock_predictor):
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        response = await client.post(
            "/predict",
            json={"summary": "Fix login bug", "description": "Users cannot login"},
        )
    assert response.status_code == 200
    body = response.json()
    assert body["predicted_seconds"] == 7200
    assert body["model_type"] == "scratch_ridge"
    mock_predictor.predict.assert_called_once_with(
        "Fix login bug", "Users cannot login"
    )

@pytest.mark.asyncio
async def test_predict_no_description(mock_predictor):
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        response = await client.post(
            "/predict",
            json={"summary": "Fix login bug"},
        )
    assert response.status_code == 200
    mock_predictor.predict.assert_called_once_with("Fix login bug", None)

@pytest.mark.asyncio
async def test_predict_no_active_model(no_model_predictor):
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        response = await client.post(
            "/predict",
            json={"summary": "Fix login bug"},
        )
    assert response.status_code == 503

@pytest.mark.asyncio
async def test_predict_missing_summary():
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        response = await client.post(
            "/predict",
            json={"description": "No summary provided"},
        )
    assert response.status_code == 422 