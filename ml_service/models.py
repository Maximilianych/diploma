import numpy as np
from sklearn.linear_model import Ridge
from sklearn.metrics import r2_score


def compute_metrics(y_true_minutes, y_pred_minutes):
    """Метрики на оригинальной шкале (минуты)"""
    y_pred_minutes = np.maximum(y_pred_minutes, 0)

    r2 = float(r2_score(y_true_minutes, y_pred_minutes))

    abs_errors = np.abs(y_true_minutes - y_pred_minutes)
    medae = float(np.median(abs_errors))

    # MdAPE — только для ненулевых y_true
    mask = y_true_minutes > 0
    if mask.sum() > 0:
        ape = np.abs((y_true_minutes[mask] - y_pred_minutes[mask]) / y_true_minutes[mask]) * 100
        mdape = float(np.median(ape))
    else:
        mdape = float("inf")

    return {"r2": r2, "medae": medae, "mdape": mdape}


class BaselineModel:
    """Предсказывает медиану обучающей выборки"""

    def __init__(self):
        self.median_log = None

    def fit(self, y_train_log):
        self.median_log = float(np.median(y_train_log))

    def predict(self, n_samples):
        return np.full(n_samples, self.median_log)


def train_ridge(X_train, y_train_log, alpha, sample_weight=None):
    """Обучает Ridge и возвращает модель"""
    model = Ridge(alpha=alpha)
    model.fit(X_train, y_train_log, sample_weight=sample_weight)
    return model


def predict_and_evaluate(model, X_val, y_val_log):
    """Предсказывает и считает метрики на оригинальной шкале"""
    if hasattr(model, "predict"):
        y_pred_log = model.predict(X_val)
    else:
        # BaselineModel
        y_pred_log = model.predict(X_val.shape[0])

    y_pred_minutes = np.expm1(y_pred_log)
    y_true_minutes = np.expm1(y_val_log)

    metrics = compute_metrics(y_true_minutes, y_pred_minutes)
    return metrics