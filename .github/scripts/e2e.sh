#!/bin/bash

CURR_DIR=`pwd`

# Start the database
docker-compose up database -d

# Reconfigure proxy configuration
sed -i 's/"host": "database"/"host": "localhost_!"/' ./proxy.conf 

# Run the proxy and capture its pid
cargo run &&
PID=$!

# Run the npm e2e test
cd lib/npm/
npm run test || touch "$CURR_DIR/failed.out"

# Clean up
cd "$CURR_DIR"
docker-compose down
kill -9 $PID

# Report result
if [ -f "failed.out" ]; then  
  exit 1
fi
