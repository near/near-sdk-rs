#!/bin/bash

cargo test --no-run --message-format=json | jq -r "select(.profile.test == true) | .filenames[]"