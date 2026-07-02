#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$script_dir/lib/repo-root.sh"
repo_root="$(tovuk_repo_root "$script_dir")"
cd "$repo_root"

: "${MINTLIFY_ADMIN_API_KEY:?MINTLIFY_ADMIN_API_KEY is required.}"
: "${MINTLIFY_PROJECT_ID:?MINTLIFY_PROJECT_ID is required.}"

mintlify_api() {
  local method="$1"
  local url="$2"
  local label="$3"
  local response_file
  local curl_exit
  local http_code

  response_file="$(mktemp "${RUNNER_TEMP:-/tmp}/mintlify-response.XXXXXX")"
  curl_exit=0
  http_code="$(
    curl -sS \
      --request "$method" \
      --url "$url" \
      --header "Authorization: Bearer ${MINTLIFY_ADMIN_API_KEY}" \
      --output "$response_file" \
      --write-out '%{http_code}'
  )" || curl_exit=$?

  if [ "$curl_exit" -ne 0 ]; then
    echo "::error title=Mintlify API request failed::curl exited ${curl_exit} while calling ${label}." >&2
    print_mintlify_response_body "$response_file"
    exit "$curl_exit"
  fi

  case "$http_code" in
    2*)
      cat "$response_file"
      ;;
    401 | 403)
      echo "::error title=Mintlify authentication failed::Mintlify rejected MINTLIFY_ADMIN_API_KEY or MINTLIFY_PROJECT_ID while calling ${label}; rotate the GitHub secret, then rerun Deploy Mintlify docs." >&2
      print_mintlify_response_body "$response_file"
      exit 1
      ;;
    *)
      echo "::error title=Mintlify API request failed::Mintlify returned HTTP ${http_code} while calling ${label}." >&2
      print_mintlify_response_body "$response_file"
      exit 1
      ;;
  esac
}

print_mintlify_response_body() {
  local response_file="$1"

  if [ ! -s "$response_file" ]; then
    return
  fi

  echo "Mintlify response body:" >&2
  if jq -c . "$response_file" >/dev/null 2>&1; then
    jq -c . "$response_file" >&2
  else
    head -c 4096 "$response_file" >&2
    echo >&2
  fi
}

trigger_deployment() {
  local response

  response="$(
    mintlify_api \
      POST \
      "https://api.mintlify.com/v1/project/update/${MINTLIFY_PROJECT_ID}" \
      "project update"
  )"
  printf '%s\n' "$response" | jq -er '.statusId'
}

wait_for_deployment() {
  local status_id="$1"
  local response
  local status
  local summary

  for attempt in $(seq 1 60); do
    response="$(
      mintlify_api \
        GET \
        "https://api.mintlify.com/v1/project/update-status/${status_id}" \
        "deployment status"
    )"
    status="$(printf '%s\n' "$response" | jq -r '.status // "unknown"')"
    summary="$(printf '%s\n' "$response" | jq -r '.summary // ""')"
    printf 'Mintlify deployment poll attempt %s/60\n' "$attempt"
    print_status "$status" "$summary"

    case "$status" in
      success)
        verify_public_agent_readiness
        exit 0
        ;;
      failure)
        if generated_subdomain_revalidation_failed "$summary" &&
          content_update_finished "$response"; then
          echo "::warning::Mintlify updated docs content but failed to revalidate an external generated subdomain. Custom-domain docs content is live."
          verify_public_agent_readiness
          exit 0
        fi
        printf '%s\n' "$response" | print_sanitized_logs
        exit 1
        ;;
    esac

    sleep 10
  done

  echo "Timed out waiting for Mintlify deployment." >&2
  exit 1
}

print_status() {
  local status="$1"
  local summary="$2"

  if generated_subdomain_revalidation_failed "$summary"; then
    printf 'Mintlify deployment status: %s external generated subdomain revalidation failed after content update\n' "$status"
  else
    printf 'Mintlify deployment status: %s %s\n' "$status" "$summary"
  fi
}

generated_subdomain_revalidation_failed() {
  local summary="$1"

  [[ "$summary" == "Failed to revalidate subdomain:"* ]]
}

print_sanitized_logs() {
  jq -r '.logs[]?' | while IFS= read -r line; do
    if generated_subdomain_revalidation_failed "$line"; then
      echo "Failed to revalidate external generated subdomain"
    else
      printf '%s\n' "$line"
    fi
  done
}

content_update_finished() {
  printf '%s\n' "$1" | jq -e '
    [.logs[]?] |
    any(. == "Successfully updated deployment") and
    any(. == "Successfully saved config") and
    any(test("^Successfully indexed [0-9]+ page\\(s\\)\\.$"))
  ' >/dev/null
}

verify_public_agent_readiness() {
  local target

  target="${TOVUK_DOCS_PUBLIC_URL:-https://docs.tovuk.com}"
  export TOVUK_DOCS_CHECK_RETRIES="${TOVUK_DOCS_CHECK_RETRIES:-12}"
  export TOVUK_DOCS_CHECK_RETRY_DELAY_MS="${TOVUK_DOCS_CHECK_RETRY_DELAY_MS:-10000}"
  printf 'Checking Mintlify public agent readiness at %s\n' "$target"
  ./scripts/check-public-contracts.sh mintlify-agent-readiness "$target"
}

wait_for_deployment "$(trigger_deployment)"
