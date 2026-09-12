#!/bin/sh

marker="${TMPDIR:-/tmp}/arklog-reconnect-after-discovery-error-$PPID"

if [ "$1 $2 $3" = "list targets -v" ]; then
  printf 'Connect server failed: discovery unavailable\n' >&2
  exit 1
fi

if [ "$1 $2 $3" = "-t CACHED-ONLINE hilog" ]; then
  if [ ! -f "$marker" ]; then
    : > "$marker"
    printf 'before-disconnect\n'
    exit 7
  fi
  printf 'after-reconnect\n'
  exec sleep 30
fi

printf 'unexpected command: %s\n' "$*" >&2
exit 1
