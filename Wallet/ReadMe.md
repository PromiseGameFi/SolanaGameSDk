Clone repo
pip install -r requirements.txt

-split secret
python main.py generate --total 5 --threshold 3

-crea
python main.py partial_sig --index 1 --to 0xRecipientAddress --value 0.01
python main.py partial_sig --index 2 --to 0xRecipientAddress --value 0.01
python main.py partial_sig --index 3 --to 0xRecipientAddress --value 0.01

python main.py send_tx --indices 1 2 3