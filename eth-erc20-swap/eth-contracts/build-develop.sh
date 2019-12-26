rm -r build/
truffle compile

truffle migrate --reset --network development --show-events

truffle test test/ERC20Test.js
