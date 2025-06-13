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
                Summarize the main points discussed. Always specify who said what.
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
}