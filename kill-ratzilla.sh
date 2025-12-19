#!/bin/bash

set -euo pipefail

# kill processes on port 8080
if command -v lsof >/dev/null 2>&1; then
    pids=$(lsof -ti:8080 2>/dev/null || true)
    if [ -n "$pids" ]; then
        echo "killing processes on port 8080: $pids"
        echo "$pids" | xargs kill -9 2>/dev/null || true
    else
        echo "no processes found on port 8080"
    fi
elif command -v fuser >/dev/null 2>&1; then
    if fuser 8080/tcp >/dev/null 2>&1; then
        echo "killing processes on port 8080"
        fuser -k 8080/tcp 2>/dev/null || true
    else
        echo "no processes found on port 8080"
    fi
else
    echo "warning: neither lsof nor fuser found, skipping port 8080 check"
fi

# kill ratzilla processes
ratzilla_pids=$(pgrep -f ratzilla 2>/dev/null || true)
if [ -n "$ratzilla_pids" ]; then
    echo "killing ratzilla processes: $ratzilla_pids"
    echo "$ratzilla_pids" | xargs kill -9 2>/dev/null || true
else
    echo "no ratzilla processes found"
fi

# also check for trunk processes (used to serve ratzilla)
trunk_pids=$(pgrep -f "trunk serve" 2>/dev/null || true)
if [ -n "$trunk_pids" ]; then
    echo "killing trunk serve processes: $trunk_pids"
    echo "$trunk_pids" | xargs kill -9 2>/dev/null || true
fi

echo "done"












