Clone repo
pip install -r requirements.txt

-split secret
python script.py generate --total 5 --threshold 3

-crea
python script.py partial_sig --index 1
python script.py partial_sig --index 2
python script.py partial_sig --index 3

python script.py send_tx --to 0xRecipientAddress --value 0.01 --indices 1 2 3 --threshold 3