#!/usr/bin/env bash
set -x
set -eo pipefail

docker stop postgres_zero2prod_axum_diesel
docker rm postgres_zero2prod_axum_diesel
