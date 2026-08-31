#!/bin/sh
printf '%s\n' "$1 $2 $3"
if [ "$2" = "EXIT-01" ]; then
  printf 'last-before-exit\n'
  exit 7
fi
if [ "$2" = "EXIT-BURST" ]; then
  index=1
  while [ "$index" -le 1000 ]; do
    printf 'exit-burst-%s\n' "$index"
    index=$((index + 1))
  done
  exit 7
fi
if [ "$2" = "TAIL-01" ]; then
  index=1
  while [ "$index" -le 49 ]; do
    printf 'bulk-%s\n' "$index"
    index=$((index + 1))
  done
  printf 'tail-before-stop\n'
  exec sleep 30
fi
printf 'first\nsecond\n'
exec sleep 30
