import os
import torch

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
from textstat import textstat
from utils.output_structures import Summary
from utils.prompts import TASK_PROMPTS

load_dotenv()


class ModelHandler:
    def __init__(self, task_prompt:str):

        quant_config = BitsAndBytesConfig(
            load_in_4bit=True, bnb_4bit_compute_dtype=torch.float16
        )
        mistral_token = os.getenv("MISTRAL_TOKEN")
        self.tokenizer = AutoTokenizer.from_pretrained(
            "mistralai/Mistral-7B-Instruct-v0.3",
            token=mistral_token,
            device_map="auto",
        )
        self.model = AutoModelForCausalLM.from_pretrained(
            "mistralai/Mistral-7B-Instruct-v0.3",
            torch_dtype=torch.float16,
            device_map="auto",
            token=mistral_token,
            quantization_config=quant_config,
        )

        if self.tokenizer.pad_token_id is None:
            self.tokenizer.pad_token_id = self.tokenizer.eos_token_id
        if self.model.config.pad_token_id is None:
            self.model.config.pad_token_id = self.model.config.eos_token_id

        self.pipeline = pipeline(
            "text-generation",
            model=self.model,
            tokenizer=self.tokenizer,
            max_new_tokens=100,
            temperature=0.5,
            top_p=0.9,
            device_map="auto",
            repetition_penalty=1.2,
            do_sample=True,
            torch_dtype=torch.float16,
            pad_token_id=self.tokenizer.eos_token_id,
            return_full_text=False,
        )
        self.llm = HuggingFacePipeline(pipeline=self.pipeline)
        self._init_output_parser()
        self._init_prompt(task_prompt=task_prompt)
        if self.prompt: 
            self.chain = self.prompt | self.llm
        else:
            raise ValueError(f"Prompt for task ({task_prompt}) not found...")

    def _determine_max_tokens(self, question: str) -> int:
        """Use textstat to evaluate complexity of user query for max tokens."""
        complexity_score = textstat.flesch_reading_ease(question)

        if complexity_score > 80:
            return 200
        elif complexity_score > 60:
            return 300  # Simple
        elif complexity_score > 40:
            return 800
        elif complexity_score > 20:
            return 900
        elif complexity_score > 10:
            return 1100
        elif complexity_score > 5:
            return 1500  # Complex
        else: return 2000

    def _update_pipeline(self, message_history) -> None:
        max_new_tokens = self._determine_max_tokens(message_history)
        self.pipeline = pipeline(
            "text-generation",
            model=self.model,
            tokenizer=self.tokenizer,
            max_new_tokens=max_new_tokens,
            temperature=0.7,
            top_p=1.0,
            device_map="auto",
            repetition_penalty=1.2,
            do_sample=True,
            torch_dtype=torch.float16,
            pad_token_id=self.tokenizer.eos_token_id,
            return_full_text=False,
        )

        self.llm = HuggingFacePipeline(pipeline=self.pipeline)
        if self.prompt: 
            self.chain = self.prompt | self.llm
        print(f"Max New Tokens updated to: \n{max_new_tokens}")

    def _init_prompt(self, task_prompt: str) -> None:
        """Initializes the prompt to be used by the model."""
        if TASK_PROMPTS.get(task_prompt):
            self.prompt = TASK_PROMPTS.get(task_prompt)
        else:
            self.prompt = None

    def _init_output_parser(
        self,
    ) -> None:
        self.output_parser = PydanticOutputParser(pydantic_object=Summary)

    def generate_response(self, message_history: str) -> str:
        """Runs model pipeline & returns response."""
        self._update_pipeline(message_history=message_history)
        print(self.pipeline._forward_params)
        response = self.chain.invoke({"message_history": {message_history}})
        return response
