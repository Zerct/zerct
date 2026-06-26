---
name: tovuk
description: Use Tovuk public-data scraper APIs with `tovuk`.
---

# Tovuk

Use when a user wants to create Tovuk scraper Requests, fetch stored Results,
inspect pricing or usage, manage billing, or open support workflows.

Set `TOVUK_OUTPUT=json` for agent sessions.

```sh
tovuk account show
tovuk api-key list
tovuk api-key create "Production scraper"
tovuk api-key revoke api_key_0123456789abcdef01234567
tovuk pricing
tovuk usage
tovuk scraper list
tovuk scraper health
tovuk scraper show tiktok
tovuk request create tiktok '{"operation":"search","query":"rust programming","limit":100}'
tovuk request create github '{"operation":"codeSearch","query":"serde language:Rust","maxRepos":3,"limit":100}'
tovuk request create reddit '{"subreddit":"rust","sort":"new","limit":100}'
tovuk request create linkedin '{"operation":"post-search","query":"b2b sales","sort_type":"date_posted","author_company_urns":"1035","limit":50}'
tovuk request show request_123
tovuk request results request_123 --limit 1000
tovuk request results request_123 --limit 1000 --cursor <nextCursor>
tovuk request cancel request_123
tovuk billing checkout
tovuk billing portal
tovuk support list --limit 20
tovuk support create "Request failed" "Request failed after retry." --request-id request_123 --scraper-id tiktok --failing-command "tovuk request show request_123" --first-log-line "upstream timeout" --severity normal
tovuk support resolve ticket_0123456789abcdef0123
```

Scraper Requests are for public data only. Do not send cookies, passwords,
account tokens, private session data, private account content, private
repository credentials, or proxy URLs.

AI/API agents may open account-scoped service tickets between the authenticated
account and Tovuk through `POST /v1/support/tickets` with an account API key or
session bearer token.
Ticket responses include `created_by` for account-session versus account-API-key
attribution.

The public CLI does not deploy websites, backends, databases, workers, storage
buckets, queues, cron jobs, custom domains, secrets, or other customer
infrastructure.
