package api

import (
	"crypto/sha256"
	"encoding/hex"
	"math/bits"
	"strconv"
)

// RegisterChallenge is the response from GET /v1/auth/register-challenge.
type RegisterChallenge struct {
	Nonce      string `json:"nonce"`
	Difficulty uint   `json:"difficulty"`
	ValidUntil string `json:"valid_until"`
}

// leadingZeroBits counts leading zero bits in the hash (from the high byte).
func leadingZeroBits(hash [32]byte) uint {
	var n uint
	for _, b := range hash {
		if b == 0 {
			n += 8
		} else {
			n += uint(bits.LeadingZeros8(b))
			break
		}
	}
	return n
}

// SolvePoW finds a solution such that SHA256(nonce_bytes || valid_until_utf8 || solution_utf8)
// has at least difficulty leading zero bits. Returns the solution string.
func SolvePoW(nonceHex, validUntil string, difficulty uint) (string, error) {
	nonce, err := hex.DecodeString(nonceHex)
	if err != nil {
		return "", err
	}
	validUntilBytes := []byte(validUntil)
	for trial := uint64(0); ; trial++ {
		solution := strconv.FormatUint(trial, 10)
		h := sha256.New()
		h.Write(nonce)
		h.Write(validUntilBytes)
		h.Write([]byte(solution))
		var hash [32]byte
		copy(hash[:], h.Sum(nil))
		if leadingZeroBits(hash) >= difficulty {
			return solution, nil
		}
	}
}
