import json
from typing import Any
from pydantic import BaseModel, Field


class Summary(BaseModel):
    summary: str = Field(description="A summary of what the users discussed")

def clean_model_output(output: str) -> Any:
    cleaned_output = output.strip().strip("```")
    print("cleaned output:", "\n", cleaned_output)
    return json.loads(cleaned_output)
