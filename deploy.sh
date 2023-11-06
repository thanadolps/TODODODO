#!/usr/bin/env bash

# Define default values for user and address, using environment variables if available
USER=${TODOER_USER:-todoer}
ADDRESS=${TODOER_ADDRESS:-63.135.165.55}

# List of services
SERVICES=("account_service" "notification_service" "task_service" "viz_service")

echo "Building.."
cargo build --release
echo "Stripping.."
for service in "${SERVICES[@]}"; do
  strip "./target/release/$service"
done


echo "Stopping docker compose on server..."
ssh "$USER@$ADDRESS" "cd app && docker compose down"

echo "Uploading..."
sftp "$USER@$ADDRESS" <<EOF
cd app || exit
put -r docker-entrypoint-initdb.d docker-entrypoint-initdb.d
put docker-compose.prod.yaml docker-compose.yaml

for service in "${SERVICES[@]}"; do
  put "./target/release/$service" "${service}/${service}"
done
EOF

echo "Starting docker compose on server..."
ssh "$USER@$ADDRESS" "cd app && docker compose up -d"

echo "Done!"