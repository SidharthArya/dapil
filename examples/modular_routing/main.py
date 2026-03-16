import sys
import os
import logging
# Add current directory to path so it can find users module
sys.path.append(os.path.dirname(__file__))

logging.basicConfig(level=logging.INFO)

from dapil import App
from users import router as users_router

app = App()

@app.get("/")
def home():
    return "Modular App Home"

# Include router with an additional prefix
app.include_router(users_router, prefix="/api/v1")

if __name__ == "__main__":
    app.serve()
