#!/bin/bash

# This script performs a small integration test on the example game using
# surena.

set -eEuo pipefail

if [[ $# -ne 2 ]]; then
	echo "Usage: $0 <PATH_TO_SURENA> <PATH_TO_EXAMPLE_LIB>" >&2
	exit 2
fi

INPUT="\
/load_plugin $2
/create std O \"10 5\"
/pov 1
5
/pov 2
invalid
4
1
/pov 1
2
1
/print
/destroy
/exit"

OUTPUT="$(echo "$INPUT" | $1 repl)"
grep -E '> B 0$' <<<"$OUTPUT" >/dev/null || (
	echo "$OUTPUT"
	echo "Got unexpected output from surena!" >&2
	exit 1
)
