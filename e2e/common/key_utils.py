import hashlib
import re


def validate_scoped_key_checksum(token: str, prefix: str) -> None:
    """
    Validate `{prefix}{64hex_secret}_{4hex_checksum}` structure and checksum.

    Backend rule:
      checksum == first4hex(SHA256(prefix || hex_secret))
    """

    if not isinstance(token, str):
        raise AssertionError("token must be a string")
    if not token.startswith(prefix):
        raise AssertionError(f"token must start with {prefix!r}")

    rest = token[len(prefix) :]
    idx = rest.rfind("_")
    if idx < 0:
        raise AssertionError("token missing '_' separator before checksum")

    hex_body = rest[:idx]
    checksum = rest[idx + 1 :]

    if len(hex_body) != 64 or not re.fullmatch(r"[0-9a-f]{64}", hex_body):
        raise AssertionError("token secret must be 64 lowercase hex chars")
    if len(checksum) != 4 or not re.fullmatch(r"[0-9a-f]{4}", checksum):
        raise AssertionError("token checksum must be 4 lowercase hex chars")

    expected = hashlib.sha256((prefix + hex_body).encode("ascii")).hexdigest()[:4]
    if checksum != expected:
        raise AssertionError(f"checksum mismatch: got {checksum}, expected {expected}")

