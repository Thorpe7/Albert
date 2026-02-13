import json
import re
from typing import Any
from pydantic import BaseModel, Field


class Summary(BaseModel):
    summary: str = Field(description="A summary of what the users discussed")



def _normalize_response(parsed: dict) -> dict[str, str]:
    """Consolidate all model output into a single {"summary": "..."} dict."""
    summary = parsed.get("summary", "")

    for key in list(parsed.keys()):
        if key == "summary":
            continue
        value = parsed[key]
        if isinstance(value, list):
            items = "\n".join(
                f"- {t.strip('- ').strip()}" if isinstance(t, str) else f"- {t}"
                for t in value
            )
            summary += f"\n\n{key}:\n{items}"
        elif isinstance(value, str):
            summary += f"\n\n{key}: {value}"

    return {"summary": summary}


def clean_model_output(output: str) -> dict[str, str]:
    match = re.search(r"```(?:json)?\s*(.*?)\s*```", output, re.DOTALL)
    cleaned_output = match.group(1) if match else output.strip()
    print("cleaned output:", "\n", cleaned_output)

    # Try standard JSON parsing first (raw_decode ignores trailing text)
    try:
        decoder = json.JSONDecoder()
        idx = cleaned_output.index("{")
        result, _ = decoder.raw_decode(cleaned_output, idx)
        return _normalize_response(result)
    except (json.JSONDecodeError, ValueError):
        pass

    # Fallback: extract summary content with regex when model produces malformed JSON
    fallback = re.search(r'"summary"\s*:\s*\*{0,2}\s*"?(.*)', cleaned_output, re.DOTALL)
    if fallback:
        text = fallback.group(1)
        # Strip trailing JSON artifacts
        text = re.sub(r'\s*"\s*\}\s*$', '', text, flags=re.DOTALL)
        text = text.strip().strip('"')
        return {"summary": text}

    # Last resort: model returned plain text with no JSON structure
    return {"summary": cleaned_output}
