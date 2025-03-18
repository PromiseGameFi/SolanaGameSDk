import json
import secp256k1
from threshold_crypto import SecretKey

SHARES_DIR = "shares/"

def sign_partial(share_index, message):
    """Sign a message using a key share."""
    share_file = os.path.join(SHARES_DIR, f"share_{share_index}.json")

    if not os.path.exists(share_file):
        print("❌ Share not found!")
        return

    with open(share_file, "r") as f:
        share_data = json.load(f)

    share = SecretKey.deserialize(share_data["share"])
    partial_signature = share.sign(message.encode())

    print(f"📝 Partial Signature from Share {share_index}: {partial_signature}")
    return partial_signature

if __name__ == "__main__":
    share_index = int(input("Enter share index: "))
    message = input("Enter message to sign: ")
    sign_partial(share_index, message)
