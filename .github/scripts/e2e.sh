#!/bin/bash

CURR_DIR=`pwd`

# Reconfigure proxy configuration
sed -i 's/"host": "database"/"host": "localhost"/' ./proxy.conf 
sed -i 's/expose/ports/' ./docker-compose.yml
sed -i 's/5432/5432:5432/' ./docker-compose.yml
sed -i 's/"sql:/docker-entrypoint-initdb.d/"/"./sql:/docker-entrypoint-initdb.d/"/' ./docker-compose.yml
sed -i 's/"sql:/docker-entrypoint-initdb.d/"/"./sql:/docker-entrypoint-initdb.d/"/' ./docker-compose.yml

# Start the database
echo "Starting database..."
docker-compose up -d database
sleep 10
docker-compose logs

# Run the proxy and capture its pid
echo "Starting proxy..."
RUST_BACKTRACE=1 cargo run &
PID=$!

status=1

while [[ $status -ne 0 ]] && [[ -d "/proc/${PID}" ]]
do   
    sleep 5
    echo "Waiting for proxy to start..."
    curl -s http://localhost:3000/info > /dev/null
    status=$?
done

if [[ ! -d "/proc/${PID}" ]]
then    
    if [[ -f "log/proxy.log" ]]
    then
        cat "log/proxy.log"
    fi    
    docker-compose down
    echo "Proxy failed to start"
    echo "E2E test failed"
    exit 1
fi

# Run the npm e2e test
echo "Starting e2e test..."
cd lib/npm/
npm run test &> e2e.log || touch "${CURR_DIR}/failed.out"
echo "Outputting test results..."
cat e2e.log

cd "${CURR_DIR}"

echo ""
echo "Outputting proxy logs..."
cat "log/proxy.log"

# Clean up
docker-compose down
kill -9 $PID

# Report result
if [ -f "failed.out" ]; then
  rm failed.out
  echo "E2E test failed"
  exit 1
fi

echo "E2E test passed"
