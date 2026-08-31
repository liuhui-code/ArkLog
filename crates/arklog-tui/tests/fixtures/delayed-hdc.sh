#!/bin/sh

if [ "$1 $2 $3" = "list targets -v" ]; then
  sleep 1
  printf 'USB-DELAYED\tConnected\n'
  exit 0
fi

if [ "$1" = "-t" ] && [ "$2" = "USB-01" ] && [ "$3" = "shell" ] && \
  [ "$4" = 'hidumper -s 1201 -a "-p Faultlogger -l -d"' ]; then
  sleep 1
  printf 'Fault log list:\n******\nappfreeze-delayed.app-20010001-1756615200\n'
  printf 'Timestamp: 2026-08-31 10:00:00\nProcess: delayed.app\nReason: APP_FREEZE\n'
  printf '******\n'
  exit 0
fi

printf 'unexpected command: %s\n' "$*" >&2
exit 1
