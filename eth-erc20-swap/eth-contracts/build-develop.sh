rm -r build/
truffle compile

truffle migrate --reset --network development --show-eventst

truffle test test/ERC20Test.js
