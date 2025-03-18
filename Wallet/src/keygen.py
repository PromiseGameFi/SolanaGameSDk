import os
import json
import secp256k1
from threshold_crypto import SecretKey

SHARES_DIR = "shares/"

def generate_key_shares(total_shares, threshold):
    """Generate a private key, split it into shares, and save them."""
    os.makedirs(SHARES_DIR, exist_ok=True)

    # Generate ECDSA private key
    private_key = SecretKey.random()
    public_key = private_key.public_key()

    # Split private key into shares
    shares = private_key.split(num_shares=total_shares, threshold=threshold)

    for i, share in enumerate(shares):
        share_file = os.path.join(SHARES_DIR, f"share_{i+1}.json")
        with open(share_file, "w") as f:
            json.dump({"share": share.serialize()}, f)

    print(f"✅ Created {total_shares} key shares. Threshold: {threshold}")
    print(f"🔑 Public Key: {public_key}")

if __name__ == "__main__":
    total_shares = int(input("Enter total number of shares: "))
    threshold = int(input("Enter threshold required: "))
    generate_key_shares(total_shares, threshold)
