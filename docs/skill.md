---
name: tovuk
description: Use Tovuk public-data scraper APIs with the native Rust CLI.
license: MIT
compatibility: Requires the `tovuk` CLI.
metadata:
  author: Tovuk
  version: "0.1"
---

# Tovuk

Use this skill when a user wants to create paid public-data scraper Requests,
poll request status, fetch stored Records, inspect usage, manage billing, or
open support workflows.

```sh
export TOVUK_OUTPUT=json
tovuk account show
tovuk api-key list
tovuk api-key create "Production scraper"
tovuk api-key revoke api_key_0123456789abcdef01234567
tovuk pricing
tovuk usage
tovuk scraper list
tovuk scraper health
tovuk scraper show tiktok
tovuk request list --limit 20
tovuk request list --limit 20 --cursor <nextCursor>
tovuk request create tiktok '{"operation":"search","query":"rust programming","limit":100}'
tovuk request create github '{"operation":"codeSearch","query":"serde language:Rust","maxRepos":3,"limit":100}'
tovuk request create linkedin '{"operation":"post-search","query":"b2b sales","sort_type":"date_posted","author_company_urns":"1035","limit":50}'
tovuk request create google-maps '{"searchTerms":["coffee shops"],"locationQuery":"Seattle, WA","limit":25}'
tovuk request create amazon '{"operation":"search","query":"mechanical keyboard","limit":25}'
tovuk request show request_123
tovuk request results request_123 --limit 1000
tovuk request results request_123 --limit 1000 --cursor <nextCursor>
tovuk billing checkout plus
tovuk billing portal
tovuk support list --limit 20
tovuk support create "Request failed" "Request failed after retry." --request-id request_123 --scraper-id tiktok --failing-command "tovuk request show request_123" --first-log-line "upstream timeout" --severity normal
tovuk support resolve ticket_0123456789abcdef0123
```

Scraper requests are public data only. Do not send cookies, passwords, account
tokens, private session data, private account content, private repository
credentials, or proxy URLs.

For ecommerce Records, inspect `featureCoverage` to see whether requested
Apify-style fields were extracted, absent, partially extracted, or accepted for
compatibility. Public metadata fields can include `tags`, `keywords`,
`questionSamples`, `customerPhotoUrls`, `minimumOrderQuantity`, `rfqText`,
`resultPosition`, and `adPosition`.

Use `tovuk scraper show <scraper> --json` before large ecommerce jobs to read
the live `inputSchema`. Current ecommerce caps are 50 search terms, 200 broad
URL or product URL values, 100 seller, shop, or category URL values, 200
product IDs or ASINs, 100 category IDs, 512 characters for search text, 2,048
characters for URLs, and 128 characters for generated product or category IDs.

AI/API agents may open account-scoped service tickets between the authenticated
account and Tovuk through `POST /v1/support/tickets` with an account API key or
session bearer token.
Ticket responses include `created_by` for account-session versus account-API-key
attribution.

The public CLI does not deploy websites, backends, databases, workers, storage
buckets, queues, cron jobs, custom domains, secrets, or other customer
infrastructure.
