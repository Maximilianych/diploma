import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

from config import load_config
from predictor import Predictor
from training import run_training
from storage import get_engine, get_active_model_info

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)

config = load_config()
predictor = Predictor(config)


@asynccontextmanager
async def lifespan(app: FastAPI):
    logging.getLogger(__name__).info("ML Service started")
    yield


app = FastAPI(title="Task Time Predictor", lifespan=lifespan)


# ============ Schemas ============

class PredictRequest(BaseModel):
    summary: str
    description: str | None = None


class PredictResponse(BaseModel):
    predicted_seconds: int
    model_type: str


class TrainResponse(BaseModel):
    status: str
    run_id: int
    best_model: str | None = None
    metrics_val: dict | None = None
    metrics_test: dict | None = None
    activated: bool = False
    message: str | None = None


class StatusResponse(BaseModel):
    active_model: str | None = None
    model_candidate_id: int | None = None
    activated_at: str | None = None
    r2_val: float | None = None
    medae_val: float | None = None
    mdape_val: float | None = None


# ============ Endpoints ============

@app.post("/predict", response_model=PredictResponse)
async def predict(req: PredictRequest):
    result = predictor.predict(req.summary, req.description)
    if result is None:
        raise HTTPException(status_code=503, detail="No active model available")
    return PredictResponse(**result)


@app.post("/retrain", response_model=TrainResponse)
async def retrain():
    try:
        result = run_training(config)
        predictor.reload()
        return TrainResponse(
            status=result["status"],
            run_id=result["run_id"],
            best_model=result.get("best_model"),
            metrics_val=result.get("metrics_val"),
            metrics_test=result.get("metrics_test"),
            activated=result.get("activated", False),
            message=result.get("message"),
        )
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/status", response_model=StatusResponse)
async def status():
    engine = get_engine(config)
    info = get_active_model_info(engine)
    if info is None:
        return StatusResponse()
    return StatusResponse(
        active_model=info["model_family"],
        model_candidate_id=info["model_candidate_id"],
        activated_at=str(info["activated_at"]),
        r2_val=info["r2_val"],
        medae_val=info["medae_val"],
        mdape_val=info["mdape_val"],
    )


@app.get("/health")
async def health():
    return {"status": "ok"}