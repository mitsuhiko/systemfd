#!/bin/sh
systemfd --no-pid -s http::3001 -- cargo watch -x run
