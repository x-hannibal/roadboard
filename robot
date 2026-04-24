#!/usr/bin/env bash
exec cargo run -q -p robot --release -- "$@"
