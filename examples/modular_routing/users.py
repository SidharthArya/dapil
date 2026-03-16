
from dapil import APIRouter

router = APIRouter(prefix="/users")

@router.get("/")
def list_users():
    return "user1, user2, user3"

@router.get("/{user_id}")
def get_user(user_id):
    # Path parameter parsing isn't implemented yet, but we can test the routing
    return f"User Info {user_id}"
