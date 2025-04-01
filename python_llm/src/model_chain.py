import torch
import textstat

from typing import List
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    pipeline,
    AutoModelForSeq2SeqLM,
    AutoModelForQuestionAnswering,
)
from langchain_huggingface.llms import HuggingFacePipeline
from langchain.prompts import PromptTemplate


class ModelHandler:
    def __init__(self):
        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

        tokenizer = AutoTokenizer.from_pretrained(
            "distilbert/distilbert-base-cased-distilled-squad"
        )
        model = AutoModelForQuestionAnswering.from_pretrained(
            "distilbert/distilbert-base-cased-distilled-squad",
            device_map="auto",
        ).to(self.device)

        if tokenizer.pad_token_id is None:
            tokenizer.pad_token_id = tokenizer.eos_token_id
        if model.config.pad_token_id is None:
            model.config.pad_token_id = model.config.eos_token_id

        self.pipeline = pipeline(
            "question-answering",
            model=model,
            tokenizer=tokenizer,
            max_new_tokens=300,
            temperature=0.5,
            top_p=0.9,
            repetition_penalty=1.2,
            torch_dtype=torch.float16,
            device_map="auto",
            pad_token_id=tokenizer.eos_token_id,
            return_full_text=False,
        )
        self.llm = HuggingFacePipeline(pipeline=self.pipeline)
        self._init_prompt()
        self.chain = self.prompt | self.llm

    def _determine_max_tokens(self, question: str) -> int:
        """Use textstat to evaluate complexity of user query for max tokens."""
        complexity_score = textstat.flesch_reading_ease(question)

        if complexity_score > 60:
            return 100  # Simple
        elif complexity_score > 40:
            return 200
        elif complexity_score > 20:
            return 300
        else:
            return 500  # Complex

    def _update_pipeline(self, message_history) -> None:
        max_new_tokens = self._determine_max_tokens(message_history)
        self.pipeline = pipeline(
            "summarization",
            model=self.pipeline.model,
            tokenizer=self.pipeline.tokenizer,
            max_new_tokens=max_new_tokens,
            temperature=0.5,
            top_p=0.9,
            repetition_penalty=1.2,
            torch_dtype=torch.float16,
            device_map="auto",
            pad_token_id=self.pipeline.tokenizer.eos_token_id,
            return_full_text=False,
        )

    def _init_prompt(
        self,
    ) -> None:
        """Initializes the prompt to be used by the model."""
        self.prompt = PromptTemplate(
            template=(
                """
                You are a helpful friend who specializes in summarizing conversations and describing the sentiment of others. Here is a message history:
                ```{message_history}```
                Given the following conversation, provided a summary of what each user said.
                ```{question}```
                
                """
            ),
            input_variables=["message_history", "question"],
        )

    def generate_response(self, message_history: List) -> str:
        """Runs model pipeline & returns response."""

        question = "Can you summarize what each user said in the chat history?"
        response = self.chain.invoke(
            {"message_history": message_history, "question": question}
        )
        print(response)
