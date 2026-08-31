#!/bin/sh

if [ "$1 $2 $3" = "list targets -v" ]; then
  sleep 1
  printf 'USB-DELAYED\tConnected\n'
  exit 0
fi

if [ "$1 $2 $3 $4 $5" = "-t USB-01 shell faultloggerd --dump" ]; then
  sleep 1
  printf 'Timestamp: 2026-08-31 10:00:00\nProcess: delayed.app\nReason: APP_FREEZE\n'
  exit 0
fi

printf 'unexpected command: %s\n' "$*" >&2
exit 1
