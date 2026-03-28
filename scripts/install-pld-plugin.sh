#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PLUGIN_ROOT="${PLD_PLUGIN_ROOT:-$ROOT/../../agent-plugins/plugins/parallel-lane-dev}"
if [[ ! -d "$PLUGIN_ROOT/scripts" || ! -d "$PLUGIN_ROOT/skills" ]]; then
  echo "error: expected parallel-lane-dev plugin at: $PLUGIN_ROOT" >&2
  echo "Clone or adjust PLD_PLUGIN_ROOT (must contain scripts/ and skills/)." >&2
  exit 1
fi
mkdir -p "$ROOT/plugins/parallel-lane-dev"
ln -sfn "$(cd "$PLUGIN_ROOT/scripts" && pwd)" "$ROOT/plugins/parallel-lane-dev/scripts"
ln -sfn "$(cd "$PLUGIN_ROOT/skills" && pwd)" "$ROOT/plugins/parallel-lane-dev/skills"
echo "Linked plugins/parallel-lane-dev/scripts -> $PLUGIN_ROOT/scripts"
echo "Linked plugins/parallel-lane-dev/skills -> $PLUGIN_ROOT/skills"
