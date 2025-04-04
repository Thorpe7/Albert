import os
import torch
import textstat

from dotenv import load_dotenv
from typing import List
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    pipeline,
    BitsAndBytesConfig,
)
from langchain_huggingface.llms import HuggingFacePipeline
from langchain.prompts import PromptTemplate
from langchain.output_parsers import PydanticOutputParser

from utils.output_structures import SummaryList

load_dotenv()


class ModelHandler:
    def __init__(self):

        quant_config = BitsAndBytesConfig(
            load_in_4bit=True, bnb_4bit_compute_dtype=torch.float16
        )
        mistral_token = os.getenv("MISTRAL_TOKEN")
        tokenizer = AutoTokenizer.from_pretrained(
            "mistralai/Mistral-7B-Instruct-v0.3",
            token=mistral_token,
            device_map="auto",
        )
        model = AutoModelForCausalLM.from_pretrained(
            "mistralai/Mistral-7B-Instruct-v0.3",
            torch_dtype=torch.float16,
            device_map="auto",
            token=mistral_token,
            quantization_config=quant_config,
        )

        if tokenizer.pad_token_id is None:
            tokenizer.pad_token_id = tokenizer.eos_token_id
        if model.config.pad_token_id is None:
            model.config.pad_token_id = model.config.eos_token_id

        self.pipeline = pipeline(
            "text-generation",
            model=model,
            tokenizer=tokenizer,
            max_new_tokens=200,
            temperature=0.5,
            top_p=0.9,
            device_map="auto",
            repetition_penalty=1.2,
            do_sample=True,
            torch_dtype=torch.float16,
            pad_token_id=tokenizer.eos_token_id,
            return_full_text=False,
        )
        self.llm = HuggingFacePipeline(pipeline=self.pipeline)
        self._init_output_parser()
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
            device_map="auto",
            repetition_penalty=1.2,
            do_sample=True,
            torch_dtype=torch.float16,
            pad_token_id=self.pipeline.tokenizer.eos_token_id,
            return_full_text=False,
        )

    def _init_prompt(
        self,
    ) -> None:
        """Initializes the prompt to be used by the model."""
        self.prompt = PromptTemplate(
            template=(
                """<s>[INST]
                You are a summarization assistant. Summarize each user's main points and sentiment.

                ONLY output real JSON data based on the following example.
                DO NOT describe the format. DO NOT create a JSON schema. DO NOT explain the structure.

                Example output (this is a real output, not a schema):

                {format_instructions}

                Message history:
                {message_history}

                Summarize each user's main points and attitude in 1-2 sentences.
                Provide one summary per user.
                Output only real JSON instances.
                [/INST]"""
            ),
            input_variables=["message_history"],
            partial_variables={
                "format_instructions": self.output_parser.get_format_instructions()
            },
        )

    def _init_output_parser(
        self,
    ) -> None:
        self.output_parser = PydanticOutputParser(pydantic_object=SummaryList)

    def generate_response(self, message_history: List) -> str:
        """Runs model pipeline & returns response."""
        response = self.chain.invoke({"message_history": {message_history}})
        return response
