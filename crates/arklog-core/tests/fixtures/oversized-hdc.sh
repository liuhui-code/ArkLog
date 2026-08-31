#!/bin/sh
trap '' PIPE
i=0
while [ "$i" -lt 100 ]; do
  printf 'USB-%04d Connected with excessive diagnostic payload\n' "$i"
  i=$((i + 1))
done
sleep 2
