#!/usr/bin/env bash
# Hook: auto-format after file writes
# Runs the appropriate formatter based on file extension.

FILE="$1"

case "$FILE" in
  *.ts|*.tsx|*.js|*.jsx|*.json|*.css|*.html)
    if command -v npx &> /dev/null; then
      npx prettier --write "$FILE" 2>/dev/null
    fi
    ;;
  *.rs)
    if command -v rustfmt &> /dev/null; then
      rustfmt "$FILE" 2>/dev/null
    fi
    ;;
esac

exit 0
