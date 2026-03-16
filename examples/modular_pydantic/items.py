from dapil import APIRouter, Request
from .models import Item
import logging

logging.basicConfig(level=logging.INFO)

router = APIRouter(prefix="/items")

@router.get("/{item_id}")
async def get_item(item_id: int, q: str = "default"):
    """
    Demonstrates GET with path parameter (item_id) and query parameter (q).
    Path parameter item_id will be automatically converted to int.
    """
    return {
        "item_id": item_id,
        "q": q,
        "message": f"Fetched item {item_id} with query {q}"
    }

@router.post("/")
async def create_item(item: Item):
    """
    Demonstrates POST with a Pydantic model (Item).
    The request body will be automatically parsed and validated.
    """
    return {
        "message": "Item created successfully",
        "item": item.dict()
    }
