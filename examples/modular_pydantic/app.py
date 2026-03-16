from dapil import App
from .items import router as items_router

app = App()

# Include the modular items router
app.include_router(items_router)

@app.route("GET", "/")
def index():
    return {"message": "Welcome to the Modular Pydantic Example"}

if __name__ == "__main__":
    app.serve()
