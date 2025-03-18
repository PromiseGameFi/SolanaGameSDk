import json
import secp256k1
from threshold_crypto import Signature

def reconstruct_signature(partial_signatures):
    """Reconstruct full signature from partial signatures."""
    full_signature = Signature.aggregate(partial_signatures)
    print(f"🔏 Full Signature: {full_signature}")
    return full_signature

if __name__ == "__main__":
    parts = int(input("Enter number of shares to reconstruct: "))
    signatures = []

    for i in range(parts):
        sig = input(f"Enter partial signature {i+1}: ")
        signatures.append(sig)

    reconstruct_signature(signatures)
