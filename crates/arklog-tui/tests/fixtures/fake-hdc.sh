#!/bin/sh

if [ "$1 $2 $3" = "list targets -v" ]; then
  printf 'USB-01\tConnected\n'
  exit 0
fi

if [ "$1 $2 $3 $4 $5" = "-t USB-01 shell faultloggerd --dump" ]; then
  printf 'Timestamp: 2026-08-30 20:10:01\nProcess: com.example.first\nReason: JS_ERROR\n\n'
  printf 'Timestamp: 2026-08-30 20:11:02\nProcess: com.example.second\nReason: APP_FREEZE\n'
  exit 0
fi

if [ "$1 $2 $3" = "-t USB-01 hilog" ]; then
  printf 'first\nsecond\n'
  exec sleep 30
fi

printf 'unexpected command: %s\n' "$*" >&2
exit 1
