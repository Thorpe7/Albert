import json
from pydantic import BaseModel, Field


class UserSummary(BaseModel):
    author: str = Field(description="The name of the user")
    summary: str = Field(description="A 1-2 sentence summary of what the user said")


class SummaryList(BaseModel):
    summaries: list[UserSummary]


def clean_model_output(output: str) -> dict:
    cleaned_output = output.strip().strip("```")
    print("cleaned output:", "\n", cleaned_output)
    return json.loads(cleaned_output)
