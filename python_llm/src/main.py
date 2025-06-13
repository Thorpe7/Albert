"""Main script for python LLM of Albert"""

import json
import argparse
from model_chain import ModelHandler
from utils.output_structures import clean_model_output

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Inputs to python llm model")
    parser.add_argument("input_file_path", help="String file path of input file")
    args = parser.parse_args()
    TestHandler = ModelHandler()
    
    with open(args.input_file_path, "r") as fp:
        messages = fp.readlines()
        message_history = "".join(messages)

    response = TestHandler.generate_response(message_history=message_history)
    print("initial response:", "\n", response, "\n")
    cleaned_response = clean_model_output(response)

    dir_id = args.input_file_path.split("/")[0]
    with open(f"{dir_id}/model_response.json", "w") as out_file:
        json.dump(cleaned_response, out_file)
