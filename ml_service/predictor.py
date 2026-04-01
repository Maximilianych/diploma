import logging

import numpy as np

from features import combine_blocks
from storage import get_engine, get_active_model_info, load_artifact

logger = logging.getLogger(__name__)


class Predictor:
    """Загружает активную модель и делает предсказания."""

    def __init__(self, config):
        self.config = config
        self.artifact = None
        self.model_info = None
        self.reload()

    def reload(self):
        """Перезагружает активную модель из БД и файловой системы."""
        try:
            engine = get_engine(self.config)
            info = get_active_model_info(engine)
            if info is None:
                logger.info("No active model found")
                self.artifact = None
                self.model_info = None
                return

            artifact = load_artifact(info["artifact_path"])
            if artifact is None:
                logger.warning("Active model artifact not found: %s", info["artifact_path"])
                self.artifact = None
                self.model_info = None
                return

            self.artifact = artifact
            self.model_info = info
            logger.info("Loaded active model: %s (candidate %d)",
                         info["model_family"], info["model_candidate_id"])
        except Exception as e:
            logger.exception("Failed to load model")
            self.artifact = None
            self.model_info = None

    def predict(self, summary: str, description: str | None = None) -> dict | None:
        """Предсказывает время выполнения задачи.

        Returns:
            dict с predicted_seconds и model_type, или None если модель недоступна.
        """
        if self.artifact is None:
            return None

        description = description or ""
        family = self.artifact["family"]

        if family == "baseline":
            pred_log = self.artifact["model"].predict(1)[0]
        else:
            extractor = self.artifact["extractor"]
            weights = self.artifact["weights"]

            X_s, X_d, X_c = extractor.transform_blocks([summary], [description])
            X = combine_blocks(X_s, X_d, X_c, weights)

            pred_log = self.artifact["model"].predict(X)[0]

        # Обратное преобразование: expm1 → минуты → секунды
        pred_minutes = float(np.expm1(pred_log))

        # Clamp
        clamp_min = self.config["prediction"]["clamp_min_minutes"]
        pred_minutes = max(pred_minutes, clamp_min)

        # Округление
        round_to = self.config["prediction"]["round_minutes_to"]
        pred_minutes = round(pred_minutes / round_to) * round_to

        pred_seconds = int(pred_minutes * 60)

        return {
            "predicted_seconds": pred_seconds,
            "model_type": family,
        }