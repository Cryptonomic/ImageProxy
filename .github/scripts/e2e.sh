#!/bin/bash

CURR_DIR=`pwd`

# Reconfigure proxy configuration
sed -i 's/"host": "database"/"host": "localhost"/' ./proxy.conf 
sed -i 's/expose/ports/' ./docker-compose.yml
sed -i 's/5432/5432:5432/' ./docker-compose.yml


# Start the database
echo "Starting database..."
docker-compose up -d database
sleep 10

# Run the proxy and capture its pid
echo "Starting proxy..."
RUST_BACKTRACE=1 cargo run &
PID=$!

status=1

while [[ $status -ne 0 ]] && [[ -d "/proc/${PID}" ]]
do   
    sleep 5
    echo "Waiting for proxy to start..."
    curl -s http://localhost:3000/info
    status=$?
done

if [[ -d "/proc/${PID}" ]]
    echo "Proxy failed to start"
    echo "E2E test failed"
    if [[ -f "log/proxy.log" ]]
        cat "log/proxy.log"
    fi
    exit 1
fi

# Run the npm e2e test
echo "Starting e2e test..."
cd lib/npm/
npm run test || touch "${CURR_DIR}/failed.out"

# Clean up
cd "${CURR_DIR}"
docker-compose down
kill -9 $PID

# Report result
if [ -f "failed.out" ]; then  
  echo "E2E test failed"
  exit 1
fi

echo "E2E test passed"
