#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

scan_roots=()
for path in tests src crates; do
  if [[ -e "$path" ]]; then
    scan_roots+=("$path")
  fi
done

failures=0

report_failure() {
  printf 'taxonomy violation: %s\n' "$1" >&2
  failures=1
}

if ((${#scan_roots[@]} > 0)); then
  bare_ignores="$(rg -n '^\s*#\[ignore\]\s*$' "${scan_roots[@]}" 2>/dev/null || true)"
  if [[ -n "$bare_ignores" ]]; then
    report_failure "bare #[ignore] is not allowed; add an explicit lane reason"
    printf '%s\n' "$bare_ignores" >&2
  fi
fi

while IFS= read -r file; do
  base="$(basename "$file")"

  if [[ "$base" != sim_* ]]; then
    if rg -q 'OPENAKTA_INTEGRATION_TEST|SKIP_INTEGRATION_TESTS|SKIP_LONG_TESTS' "$file"; then
      report_failure "runtime env skips are not allowed in default lanes: $file"
    fi

    if rg -q 'std::env::var\(|env::var\(' "$file" && rg -q 'return Ok\(\(\)\);|return;' "$file"; then
      report_failure "runtime self-skip pattern detected in $file"
    fi
  fi

  if [[ "$base" == *integration*.rs && "$base" != sim_* ]]; then
    if rg -qi 'mock server|mocked|wiremock-style|mock response|simulation-only' "$file"; then
      report_failure "simulation file must use sim_ prefix instead of integration name: $file"
    fi
  fi
done < <(
  find . \
    \( -path './.git' -o -path './target' -o -path '*/target' \) -prune -o \
    \( -path './tests/*.rs' -o -path '*/tests/*.rs' \) -print | sort
)

if ((failures != 0)); then
  exit 1
fi

printf 'test taxonomy verification passed\n'
