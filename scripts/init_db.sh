#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v diesel)" ]; then
    echo >&2 "Error: diesel_cli is not installed."
    echo >&2 "Use:"
    echo >&2 "    cargo install diesel_cli --locked --no-default-features --features postgres"
    echo >&2 "to install it."
    exit 1
fi

# Check if a custom parameter has been set, otherwise use default values
DB_PORT="${POSTGRES_PORT:=5432}"
SUPERUSER="${SUPERUSER:=postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:=password}"

APP_USER="${APP_USER:=app}"
APP_USER_PWD="${APP_USER_PWD:=secret}"
APP_DB_NAME="${APP_DB_NAME:=newsletter}"

# Launch postgres using Docker
CONTAINER_NAME="postgres_zero2prod_axum_diesel"
docker run \
    -e POSTGRES_USER="$SUPERUSER" \
    -e POSTGRES_PASSWORD="$SUPERUSER_PWD" \
    --health-cmd="pg_isready -U ${SUPERUSER} || exit 1" \
    --health-interval=1s \
    --health-timeout=5s \
    --health-retries=5 \
    -p "$DB_PORT:5432" \
    -d \
    --name "${CONTAINER_NAME}" \
    postgres:14-alpine -N 1000
    # ^ Increased maximum number of connections for testing purposes

# Wait for Postgres to be ready to accept connections
until [ \
    "$(docker inspect -f "{{.State.Health.Status}}" ${CONTAINER_NAME})" == \
    "healthy" \
]; do
    >&2 echo "Waiting for Postgres to be healthy...- sleeping for 1 second"
    sleep 1
done

>&2 echo "Postgres is healthy and ready to accept connections on PORT ${DB_PORT}!"

# Create the application user
CREATE_QUERY="CREATE USER ${APP_USER} WITH PASSWORD '${APP_USER_PWD}';"
docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_QUERY}"

# Grant create db privileges to the app user
GRANT_QUERY="ALTER USER ${APP_USER} CREATEDB;"
docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${GRANT_QUERY}"

DATABASE_URL="postgres://${APP_USER}:${APP_USER_PWD}@localhost:${DB_PORT}/${APP_DB_NAME}"
export DATABASE_URL

diesel setup
