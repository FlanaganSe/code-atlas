#!/usr/bin/env bash
# Hook: block dangerous commands before execution
# Add patterns here that should be blocked outright.

COMMAND="$1"

BLOCKED_PATTERNS=(
  "rm -rf /"
  "rm -rf ~"
  "DROP DATABASE"
  "DROP TABLE"
  "> /dev/sda"
  "mkfs."
  ":(){:|:&};:"
)

for pattern in "${BLOCKED_PATTERNS[@]}"; do
  if [[ "$COMMAND" == *"$pattern"* ]]; then
    echo "BLOCKED: Command contains dangerous pattern: $pattern"
    exit 1
  fi
done

exit 0
