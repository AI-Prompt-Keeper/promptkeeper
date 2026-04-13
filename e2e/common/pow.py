import hashlib


def leading_zero_bits(hash_bytes: bytes) -> int:
    """
    Count leading zero bits in the hash, from MSB of the first byte.
    Matches the semantics of the backend PoW difficulty checks.
    """

    n = 0
    for b in hash_bytes:
        if b == 0:
            n += 8
        else:
            # number of leading zeros within the byte
            n += 8 - b.bit_length()
            break
    return n


def solve_pow(nonce_hex: str, valid_until: str, difficulty: int) -> str:
    """
    Finds a solution string such that:
      SHA256(nonce_bytes || valid_until_utf8 || solution_utf8)
    has at least `difficulty` leading zero bits.
    """

    nonce = bytes.fromhex(nonce_hex)
    vu_bytes = valid_until.encode("utf-8")
    prefix = nonce + vu_bytes

    trial = 0
    difficulty = int(difficulty)
    while True:
        solution = str(trial).encode("utf-8")
        h = hashlib.sha256(prefix + solution).digest()
        if leading_zero_bits(h) >= difficulty:
            return solution.decode("utf-8")
        trial += 1

