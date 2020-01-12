truffle migrate --reset --network main

rm -rfv contract-dist/ERC20HTLC.json

cp -rfv build/contracts/ERC20HTLC.json contract-dist/
