#!/bin/sh

if [ "$1 $2 $3" = "list targets -v" ]; then
  printf 'USB-01 Connected\n'
  sleep 8 &
  exit 0
fi

if [ "$1 $2 $3" = "-t USB-01 hilog" ]; then
  printf 'connected-log\n'
  exec sleep 30
fi

exit 1
