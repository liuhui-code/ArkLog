#!/bin/sh

if [ "$1 $2 $3" = "list targets -v" ]; then
  printf 'USB-OFF USB Offline Phone hdc-1\n'
  printf 'USB-NEW USB Ready Phone hdc-2\n'
  exit 0
fi

printf 'unexpected command: %s\n' "$*" >&2
exit 1
