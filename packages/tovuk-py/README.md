# tovuk

Python package for launching the native Tovuk Rust CLI. It installs or
downloads the same binary used by npm, Homebrew, and Cargo.

```sh
pipx install tovuk
tovuk login --json
tovuk account show --json
tovuk api-key list --json
tovuk api-key create "Production scraper" --json
tovuk api-key revoke api_key_0123456789abcdef01234567 --json
tovuk pricing --json
tovuk scraper list --json
tovuk scraper health --json
tovuk scraper show tiktok --json
tovuk request list --limit 20 --json
tovuk request list --limit 20 --cursor <nextCursor> --json
tovuk request create reddit '{"subreddit":"rust","sort":"new","limit":100}' --json
tovuk request create tiktok '{"operation":"search","query":"rust programming","limit":100}' --json
tovuk request create google-maps '{"searchTerms":["coffee shops"],"locationQuery":"Seattle, WA","limit":25}' --json
tovuk request create amazon '{"operation":"search","query":"mechanical keyboard","limit":25}' --json
tovuk request show request_123 --json
tovuk request results request_123 --limit 1000 --json
tovuk usage --json
tovuk billing checkout plus --json
tovuk billing portal --json
tovuk support create "Request failed" "Request failed after retry." --request-id request_123 --scraper-id tiktok --failing-command "tovuk request show request_123 --json" --first-log-line "upstream timeout" --json
```

Use `tovuk request list --limit <n> --cursor <nextCursor> --json` and
`tovuk request results <request_id> --limit <n> --cursor <nextCursor> --json`
to continue request and stored-result pagination.

Set `TOVUK_OUTPUT=json` for agent sessions. Set
`TOVUK_NATIVE_BINARY=/path/to/tovuk` to test a local binary.

The CLI does not deploy websites, backends, databases, workers, storage buckets,
queues, cron jobs, custom domains, secrets, or other customer infrastructure.

AI/API agents may also open account-scoped service tickets between your account
and Tovuk through `POST /v1/support/tickets` with an account API key or session
bearer token.
Ticket responses include `created_by` for account-session versus account-API-key
attribution.

Scraper requests are public data only. Do not send cookies, passwords, account
tokens, private session data, private account content, private repository
credentials, or proxy URLs.

Homebrew uses the main public repository tap:

```sh
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
```
