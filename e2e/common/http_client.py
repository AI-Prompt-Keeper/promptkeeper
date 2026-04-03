import json
import socket
import urllib.error
import urllib.request
from typing import Any, Mapping, Optional, Tuple


def _url(base_url: str, path: str) -> str:
    return base_url.rstrip("/") + path


class HttpClient:
    """
    Minimal HTTP JSON client using stdlib (no extra deps).
    """

    def __init__(self, base_url: str, timeout_s: int = 30):
        self.base_url = base_url.rstrip("/")
        self.timeout_s = timeout_s

    def get_json(
        self,
        path: str,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Tuple[int, Any]:
        url = _url(self.base_url, path)
        req = urllib.request.Request(url=url, method="GET", headers=dict(headers or {}))
        try:
            with urllib.request.urlopen(req, timeout=self.timeout_s) as resp:
                raw = resp.read().decode("utf-8")
                if not raw.strip():
                    return resp.status, None
                return resp.status, json.loads(raw)
        except urllib.error.HTTPError as e:
            raw = e.read().decode("utf-8", errors="replace")
            try:
                return e.code, json.loads(raw)
            except Exception:
                return e.code, {"error": raw}
        except (urllib.error.URLError, socket.timeout) as e:
            raise RuntimeError(f"GET {url} failed: {e}") from e

    def post_json(
        self,
        path: str,
        body: Any,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Tuple[int, Any]:
        url = _url(self.base_url, path)
        data = json.dumps(body).encode("utf-8")
        hdrs = {"Content-Type": "application/json"}
        hdrs.update(dict(headers or {}))
        req = urllib.request.Request(url=url, method="POST", headers=hdrs, data=data)
        try:
            with urllib.request.urlopen(req, timeout=self.timeout_s) as resp:
                raw = resp.read().decode("utf-8")
                if not raw.strip():
                    return resp.status, None
                return resp.status, json.loads(raw)
        except urllib.error.HTTPError as e:
            raw = e.read().decode("utf-8", errors="replace")
            try:
                return e.code, json.loads(raw)
            except Exception:
                return e.code, {"error": raw}
        except (urllib.error.URLError, socket.timeout) as e:
            raise RuntimeError(f"POST {url} failed: {e}") from e

    def patch_json(
        self,
        path: str,
        body: Any,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Tuple[int, Any]:
        url = _url(self.base_url, path)
        data = json.dumps(body).encode("utf-8")
        hdrs = {"Content-Type": "application/json"}
        hdrs.update(dict(headers or {}))
        req = urllib.request.Request(url=url, method="PATCH", headers=hdrs, data=data)
        try:
            with urllib.request.urlopen(req, timeout=self.timeout_s) as resp:
                raw = resp.read().decode("utf-8")
                if not raw.strip():
                    return resp.status, None
                return resp.status, json.loads(raw)
        except urllib.error.HTTPError as e:
            raw = e.read().decode("utf-8", errors="replace")
            try:
                return e.code, json.loads(raw)
            except Exception:
                return e.code, {"error": raw}
        except (urllib.error.URLError, socket.timeout) as e:
            raise RuntimeError(f"PATCH {url} failed: {e}") from e

    def delete(
        self,
        path: str,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Tuple[int, Any]:
        url = _url(self.base_url, path)
        req = urllib.request.Request(url=url, method="DELETE", headers=dict(headers or {}))
        try:
            with urllib.request.urlopen(req, timeout=self.timeout_s) as resp:
                raw = resp.read().decode("utf-8")
                if not raw.strip():
                    return resp.status, None
                return resp.status, json.loads(raw)
        except urllib.error.HTTPError as e:
            raw = e.read().decode("utf-8", errors="replace")
            try:
                return e.code, json.loads(raw)
            except Exception:
                return e.code, {"error": raw}
        except (urllib.error.URLError, socket.timeout) as e:
            raise RuntimeError(f"DELETE {url} failed: {e}") from e

