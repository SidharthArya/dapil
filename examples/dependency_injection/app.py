from dapil import App, Depends
import logging


logging.basicConfig(level=logging.INFO)

app = App(title="Dependency Injection Example")

def get_db_session():
    # In a real app, this would yield a database session
    return "fake_db_session"

def get_current_user(token: str, db: str = Depends(get_db_session)):
    # This dependency uses another dependency!
    return {"token": token, "db_session": db, "username": "admin"}

@app.get("/users/me")
def read_current_user(user: dict = Depends(get_current_user)):
    return {"current_user": user}

@app.get("/items")
def read_items(limit: int = 10, offset: int = 0, db: str = Depends(get_db_session)):
    return {"limit": limit, "offset": offset, "db_session": db}

if __name__ == "__main__":
    app.serve()
