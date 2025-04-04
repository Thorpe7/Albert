from pydantic import BaseModel, Field


class UserSummary(BaseModel):
    author: str = Field(description="The name of the user")
    summary: str = Field(description="A 1-2 sentence summary of what the user said")


class SummaryList(BaseModel):
    summaries: list[UserSummary]
