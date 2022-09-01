#!/bin/bash

# Start the node and save the pid

nohup ./target/release/parami --alice --chain dev --tmp --unsafe-ws-external --rpc-cors all --unsafe-rpc-external --rpc-methods Unsafe --enable-offchain-indexing true > /dev/null 2>&1 &

pid=$!

# Wait for the node to start
while ! lsof -i:9944; do
  sleep 0.1
done

# Run the tests

npm i
npm test

# Stop the node

kill -9 $pid
