#!/bin/sh

if [ "$1 $2 $3" = "list targets -v" ]; then
  printf 'USB-A USB Offline Phone hdc-1\n'
  printf 'USB-B USB Ready Phone hdc-2\n'
  exit 0
fi

if [ "$1 $2 $3" = "-t USB-A hilog" ]; then
  index=0
  while [ "$index" -lt 50 ]; do
    printf 'old-%s\n' "$index"
    index=$((index + 1))
  done
  printf 'old-tail-after-switch\n'
  exec sleep 30
fi

if [ "$1 $2 $3" = "-t USB-EXIT hilog" ]; then
  index=0
  while [ "$index" -lt 1000 ]; do
    printf 'retiring-%s\n' "$index"
    index=$((index + 1))
  done
  exit 7
fi

if [ "$1 $2 $3" = "-t USB-B hilog" ]; then
  printf 'new-device\n'
  exec sleep 30
fi

printf 'unexpected command: %s\n' "$*" >&2
exit 1
