import logging

import numpy as np
from scipy.sparse import hstack
from sklearn.feature_extraction.text import TfidfVectorizer

logger = logging.getLogger(__name__)


class FeatureExtractor:
    """TF-IDF векторизация summary, description и char n-grams."""

    def __init__(self, config):
        feat_cfg = config["features"]

        self.summary_vec = TfidfVectorizer(
            analyzer=feat_cfg["summary"]["analyzer"],
            ngram_range=tuple(feat_cfg["summary"]["ngram_range"]),
            max_features=feat_cfg["summary"]["max_features"],
            min_df=feat_cfg["summary"]["min_df"],
            max_df=feat_cfg["summary"]["max_df"],
            sublinear_tf=feat_cfg["summary"]["sublinear_tf"],
        )
        self.description_vec = TfidfVectorizer(
            analyzer=feat_cfg["description"]["analyzer"],
            ngram_range=tuple(feat_cfg["description"]["ngram_range"]),
            max_features=feat_cfg["description"]["max_features"],
            min_df=feat_cfg["description"]["min_df"],
            max_df=feat_cfg["description"]["max_df"],
            sublinear_tf=feat_cfg["description"]["sublinear_tf"],
        )
        self.char_vec = TfidfVectorizer(
            analyzer=feat_cfg["char"]["analyzer"],
            ngram_range=tuple(feat_cfg["char"]["ngram_range"]),
            max_features=feat_cfg["char"]["max_features"],
            min_df=feat_cfg["char"]["min_df"],
            max_df=feat_cfg["char"].get("max_df", 1.0),
            sublinear_tf=feat_cfg["char"]["sublinear_tf"],
        )

    def fit(self, summaries, descriptions):
        combined = [s + " " + d for s, d in zip(summaries, descriptions)]
        self.summary_vec.fit(summaries)
        self.description_vec.fit(descriptions)
        self.char_vec.fit(combined)
        logger.info(
            "Fitted vectorizers: summary=%d, description=%d, char=%d features",
            len(self.summary_vec.vocabulary_),
            len(self.description_vec.vocabulary_),
            len(self.char_vec.vocabulary_),
        )

    def transform_blocks(self, summaries, descriptions):
        """Возвращает три отдельные sparse-матрицы (без весов)."""
        combined = [s + " " + d for s, d in zip(summaries, descriptions)]
        X_s = self.summary_vec.transform(summaries)
        X_d = self.description_vec.transform(descriptions)
        X_c = self.char_vec.transform(combined)
        return X_s, X_d, X_c

    def fit_transform_blocks(self, summaries, descriptions):
        self.fit(summaries, descriptions)
        return self.transform_blocks(summaries, descriptions)


def combine_blocks(X_s, X_d, X_c, weights):
    """Применяет веса к блокам и объединяет в одну матрицу."""
    ws, wd, wc = weights
    return hstack([ws * X_s, wd * X_d, wc * X_c], format="csr")