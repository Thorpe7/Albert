"""Main script for python LLM of Albert"""

import json
from model_chain import ModelHandler
from utils.output_structures import clean_model_output

if __name__ == "__main__":
    TestHandler = ModelHandler()
    with open("chat_history.txt", "r") as fp:
        messages = fp.readlines()
        message_history = "".join(messages)

    response = TestHandler.generate_response(message_history=message_history)
    print("initial response:", "\n", response, "\n")
    cleaned_response = clean_model_output(response)

    with open("model_response.json", "w") as out_file:
        json.dump(cleaned_response, out_file)
