from typing import Optional, Union, Any

class WebSocket:
    def __init__(self, bridge: Any):
        self._bridge = bridge
        self.client_state = "CONNECTING"

    async def accept(self, subprotocol: Optional[str] = None):
        await self._bridge.accept(subprotocol)
        self.client_state = "CONNECTED"

    async def receive_text(self) -> str:
        return await self._bridge.receive_text()

    async def receive_bytes(self) -> bytes:
        return await self._bridge.receive_bytes()

    async def send_text(self, data: str):
        await self._bridge.send_text(data)

    async def send_bytes(self, data: bytes):
        await self._bridge.send_bytes(data)

    async def close(self, code: int = 1000, reason: Optional[str] = None):
        await self._bridge.close(code, reason)
        self.client_state = "DISCONNECTED"

    async def receive_json(self, mode: str = "text") -> Any:
        import json
        if mode == "text":
            data = await self.receive_text()
        else:
            data = await self.receive_bytes()
        return json.loads(data)

    async def send_json(self, data: Any, mode: str = "text"):
        import json
        text = json.dumps(data)
        if mode == "text":
            await self.send_text(text)
        else:
            await self.send_bytes(text.encode("utf-8"))
