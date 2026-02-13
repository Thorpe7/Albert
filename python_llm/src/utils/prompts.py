''' Script to house prompts for various bot functions '''

from langchain.prompts import PromptTemplate

TASK_PROMPTS = {
    "STANDARD_SUMMARY":
        PromptTemplate(
            template=(
                """<s>[INST]
                You are a summarization assistant.
                Summarize the main points discussed in a detailed and descriptive manner.
                If a particularly good point is made by a user, include what that user said. 
                Include relevant user concerns, specific examples mentioned, and highlight overall sentiment or themes. 
                Aim for a comprehensive and thoughtful summary with depth.

                If a message only contains a link, image, or GIF, summarize it as "[User shared a link]" or skip it if irrelevant.
                Do NOT try to describe or interpret links.

                Provide ONLY the summary.
                Output only real JSON instances. 
                Adhere strictly to the output schema:
                ```
                {{ "summary": "<summary of the discussion>" }}
                ```
                Message history:
                {message_history}
                [/INST]"""
            ),
            input_variables=["message_history"],
        ),
    
    "PER_USER_SUMMARY":
        PromptTemplate(
            template=(
                """<s>[INST]
                You are a summarization assistant.
                Summarize the main points discussed and Always specify who said what or who you are summarizing.
                Summaries should be detailed and descriptive and highlight overall sentiment or themes.

                If a message only contains a link, image, or GIF, summarize it as "[User shared a link]" or skip it if irrelevant.
                Do NOT try to describe or interpret links.

                Provide ONLY the summary.
                Output only real JSON instances.
                Adhere strictly to the output schema:
                ```
                {{ "summary": "<summary of the discussion>" }}
                ```
                Message history:
                {message_history}
                [/INST]"""
            ),
            input_variables=["message_history"],
        ),

    "ARTICLE_SUMMARIZATION":
        PromptTemplate(
            template=(
                """<s>[INST]
                You are an article summarization assistant.
                Summarize the following article as bullet points inside a JSON object.
                Use \\n for line breaks and - for each bullet point inside the summary string.

                Example output:
                {{ "summary": "\U0001f4cc Main Takeaway: The city council voted to ban single-use plastics.\\n\\n\U0001f4cb Key Points:\\n- The ban covers bags, straws, and containers effective January 2026.\\n- Local businesses will receive subsidies to transition to alternatives.\\n- Environmental groups praised the decision as long overdue." }}

                Follow that exact format. Be concise but informative. Each bullet should be one to two sentences.
                Output ONLY the JSON object, nothing else.

                Article content:
                {message_history}
                [/INST]"""
            ),
            input_variables=["message_history"],
        ),
}