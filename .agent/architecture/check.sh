#!/usr/bin/env bash
# 層間の依存ルール違反を検出する。ルールは SKILL.md を参照。
set -euo pipefail

cd "$(dirname "$0")/../.."

# 層 -> 依存してはいけない層(| 区切り)
declare -a RULES=(
  "domain:application|infrastructure|presentation"
  "application:infrastructure|presentation"
  "infrastructure:presentation"
  "presentation:infrastructure"
)

status=0
for rule in "${RULES[@]}"; do
  layer="${rule%%:*}"
  forbidden="${rule#*:}"
  files=()
  [[ -f "src/$layer.rs" ]] && files+=("src/$layer.rs")
  [[ -d "src/$layer" ]] && while IFS= read -r f; do files+=("$f"); done < <(find "src/$layer" -name '*.rs')
  [[ ${#files[@]} -eq 0 ]] && continue

  # `crate::x` と `crate::{x, ...}` の両形式を検出する
  pattern="crate::(${forbidden})\b|crate::\{[^}]*\b(${forbidden})\b"
  if grep -nE "$pattern" "${files[@]}"; then
    echo "違反: $layer は ${forbidden//|/, } に依存できない" >&2
    status=1
  fi
done

if [[ $status -eq 0 ]]; then
  echo "OK: 依存ルール違反なし"
fi
exit $status
