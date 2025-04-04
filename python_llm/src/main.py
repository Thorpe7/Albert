"""Main script for python LLM of Albert"""

import json
from model_chain import ModelHandler

# TODO: Change output from Rust to Python
# TODO: Add context window check mechanism
# TODO: Trial run

if __name__ == "__main__":
    # with open("/home/thorpe/git_repos/Albert/chat_history.json", "r") as fp:
    #     message_history = json.load(fp)
    TestHandler = ModelHandler()
    message_history = """
    author: **user1**; content: Kinda disagree - filibuster is like the best tool the minority party has to stop legislation from being moved along. Used tactically it can slow/stop the progression of bad legislation 🤷‍♂️
    author: **user2**; content: Has it ever worked? I can dig into it more but I feel like every time this stuff has happened it amounts to basically nothing
    But the minority party gets a lot of great clips they can post on socials so their constituents can lap it up
    author: **user3**; content: at this point I consider anything that impedes the maga agenda a win .....I don't want to draw this analogy, but like, you can win Rivals matches just by contesting the payload and running the opponent's clock out you don't even really need to kill them, as long as you stop their progress
    """
    message_history = message_history
    TestHandler.generate_response(message_history=message_history)
