#!/usr/bin/env bash

openapi-generator generate -i docs/openapi.yml -g rust-axum -o ./gen