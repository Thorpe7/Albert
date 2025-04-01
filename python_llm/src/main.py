"""Main script for python LLM of Albert"""

import os
import json
from model_chain import ModelHandler

# TODO: Langchain & HF imports
# TODO: Main chain workflow
# TODO: Output & Prompt formatting

if __name__ == "__main__":
    with open("/home/thorpe/git_repos/Albert/chat_history.json", "r") as fp:
        message_history = json.load(fp)
    TestHandler = ModelHandler()
    message_history = """
    **papermoooon**: Kinda disagree - filibuster is like the best tool the minority party has to stop legislation from being moved along. Used tactically it can slow/stop the progression of bad legislation 🤷‍♂️
    **papermoooon**: Has it ever worked? I can dig into it more but I feel like every time this stuff has happened it amounts to basically nothing
    But the minority party gets a lot of great clips they can post on socials so their constituents can lap it up
    """
    TestHandler.generate_response(message_history=message_history)
