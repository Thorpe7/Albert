"""Main script for python LLM of Albert"""

import json
import argparse

from model_chain import ModelHandler
from utils.output_structures import clean_model_output

# Necessary as Rust calling main.py as __main__
if __name__ == "__main__":
    # Parse incoming arg from rust
    parser = argparse.ArgumentParser()
    parser.add_argument("input_file", help="Passed text file from Rust")
    args = parser.parse_args()
    with open(args.input_file, "r") as fp: # Reading of file contents
        messages = fp.readlines()
        message_history = "".join(messages)
    
    # Call Model handler
    TestHandler = ModelHandler()
    response = TestHandler.generate_response(message_history=message_history)
    print("initial response:", "\n", response, "\n")
    cleaned_response = clean_model_output(response)

    # Return model output as json
    with open("model_response.json", "w") as out_file:
        json.dump(cleaned_response, out_file)
