from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI(title="Task Time Predictor")

class PredictRequest(BaseModel):
    title: str
    description: str | None = None

class PredictResponse(BaseModel):
    predicted_hours: float

@app.post("/predict", response_model=PredictResponse)
async def predict(req: PredictRequest):
    # Заглушка — всегда 0
    return PredictResponse(predicted_hours=0.0)

@app.get("/health")
async def health():
    return {"status": "ok"}