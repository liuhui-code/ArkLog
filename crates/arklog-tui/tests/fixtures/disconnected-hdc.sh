#!/bin/sh

if [ "$1 $2 $3" = "list targets -v" ]; then
  exit 0
fi

if [ "$1 $2 $3" = "-t USB-01 hilog" ]; then
  printf 'before-disconnect\n'
  exec sleep 30
fi

printf 'unexpected command: %s\n' "$*" >&2
exit 1
