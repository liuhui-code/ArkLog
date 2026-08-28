#!/bin/sh
printf '%s\n' "$1 $2 $3"
printf 'first\nsecond\n'
exec sleep 30
