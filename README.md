# Tovuk

Tovuk is a paid public-data scraper API with a native Rust CLI. Use it to list
available scrapers, create scraper requests, poll request status, fetch stored
records, inspect usage, manage billing, and contact support.

The public CLI is intentionally narrow. It does not deploy websites, backends,
databases, workers, storage buckets, queues, cron jobs, custom domains,
secrets, or other customer infrastructure.

## Install

```sh
cargo install tovuk
npm install -g tovuk
pipx install tovuk
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
```

The npm and PyPI packages are thin launchers for the same native Rust binary.
They do not ship runtime JavaScript dependencies.

## Agent Workflow

For agent sessions, set JSON output once:

```sh
export TOVUK_OUTPUT=json
```

Then use the scraper workflow:

```sh
tovuk login --json
tovuk account show --json
tovuk account activity --limit 20 --json
tovuk account update --handle tovuk-team --display-name "Tovuk Team" --json
tovuk pricing --json
tovuk scraper list --json
tovuk scraper health --json
tovuk scraper show tiktok --json
tovuk request list --limit 20 --json
tovuk request create tiktok '{"operation":"search","query":"rust programming","limit":100}' --json
tovuk request create github '{"operation":"codeSearch","query":"serde language:Rust","maxRepos":3,"limit":100}' --json
tovuk request create linkedin '{"operation":"post-search","query":"b2b sales","sort_type":"date_posted","author_company_urns":"1035","limit":50}' --json
tovuk request show request_123 --json
tovuk request results request_123 --limit 1000 --json
tovuk request cancel request_123 --json
tovuk usage --json
tovuk billing checkout --json
tovuk billing portal --json
tovuk support list --limit 20 --json
tovuk support create "Request failed" "The scraper request failed after retry. Request id: request_123. First error: upstream timeout." --failing-command "tovuk request show request_123 --json" --first-log-line "upstream timeout" --severity normal --json
tovuk support resolve ticket_0123456789abcdef0123 --json
```

## Public-Data Boundary

Scraper requests are public data only. They must use public URLs, public search terms, public profile
handles, public place ids, or public identifiers only. Do not send cookies,
passwords, account tokens, private session data, private account content,
private repository credentials, or proxy URLs through the public API or CLI.

Use `tovuk pricing --json` and `tovuk usage --json` before high-count requests.
Pricing responses expose exact scraper prices in `priceEvents[].usdMicros`.
Usage responses expose current-month estimates in
`usage.billingEstimate.lineItems`.

## API

Primary routes:

```http
GET /health
GET /healthz
GET /v1/status
POST /v1/login/device
GET /v1/login/device/{device_code}
GET /v1/account
PATCH /v1/account
GET /v1/account/activity
GET /v1/scrapers
GET /v1/scrapers/health
GET /v1/scrapers/{scraper}
GET /v1/requests
POST /v1/requests
GET /v1/requests/{request_id}
POST /v1/requests/{request_id}/cancel
GET /v1/requests/{request_id}/results
GET /v1/usage
POST /v1/billing/checkout
POST /v1/billing/portal
GET /v1/support/tickets
POST /v1/support/tickets
POST /v1/support/tickets/{ticket_id}/resolve
```

## Development

```sh
./scripts/check-all.sh
```

The full check builds the native CLI, runs Rust tests and strict Clippy, checks
package wrappers, validates docs and OpenAPI, and confirms retired deploy and
resource commands remain unavailable.
