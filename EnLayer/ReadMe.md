

cargo run -- split --key-file-path ./private_key.json --total-shares 3 --threshold 2 --output-dir ./shares

short option: 

cargo run -- split -k ./private_key.json -s 3 -t 2 -o ./shares

cargo run -- sign --share-path ./shares/share_1.json --to 0x868263F7D8B5339655E7B3C9097c7b5149099AAf --value 0.01 --share-index 1

short function

cargo run -- sign -p ./shares/share_1.json -t 0x868263F7D8B5339655E7B3C9097c7b5149099AAf -v 0.01 -i 1

cargo run -- send-tx --signatures signature_1.json,signature_2.json --to 0x868263F7D8B5339655E7B3C9097c7b5149099AAf --value 0.01

short function

cargo run -- send-tx -s signature_1.json,signature_2.json -t 0x868263F7D8B5339655E7B3C9097c7b5149099AAf -v 0.01


cargo run -- split -k ./private_key.json -s 3 -t 2 -o ./shares

cargo run -- sign -p ./shares/share_1.json -t 0x868263F7D8B5339655E7B3C9097c7b5149099AAf -v 0.01 -i 1 -n 3 -r 2
cargo run -- sign -p ./shares/share_2.json -t 0x868263F7D8B5339655E7B3C9097c7b5149099AAf -v 0.01 -i 2 -n 3 -r 2


cargo run -- send-tx -s signature_1.json,signature_2.json -t 0x868263F7D8B5339655E7B3C9097c7b5149099AAf -v 0.01

cargo run -- sign -p ./shares/share_2.json -t 0x868263F7D8B5339655E7B3C9097c7b5149099AAf -v 0.01 -i 2 --total-shares 3 --threshold 2