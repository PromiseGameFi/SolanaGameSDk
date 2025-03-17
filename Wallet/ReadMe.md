Clone repo
python -m venv venv
pip install -r requirements.txt

# Generate Key Shares
python main.py generate --total 5 --threshold 3

# Using network gas price (recommended)
python main.py sign --share 1 --to 0x123abc... --value 0.01

# Manual override
python main.py sign --share 1 --to 0x123abc... --value 0.01 --gas-price 25

# Reconstruct Full Signature
python main.py reconstruct --shares 1 2 3

# Send Transaction
python main.py send

# wallet info
python main.py info