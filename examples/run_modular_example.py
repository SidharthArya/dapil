import sys
import os

# Add project root to sys.path so we can import from examples.modular_pydantic
project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

from examples.modular_pydantic.app import app

if __name__ == "__main__":
    print("Starting Modular Pydantic Example Server...")
    app.serve()
