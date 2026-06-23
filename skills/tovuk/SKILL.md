---
name: tovuk
description: Use Tovuk public-data scraper APIs with `tovuk`.
---

# Tovuk

Use when a user wants to create Tovuk scraper Requests, fetch stored Results,
inspect pricing or usage, manage billing, or open support and abuse workflows.

Set `TOVUK_OUTPUT=json` for agent sessions.

```sh
tovuk account show
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
tovuk request cancel request_123
tovuk billing checkout
tovuk billing portal
tovuk support list --limit 20
tovuk support create "Request failed" "Request id request_123 failed after retry." --failing-command "tovuk request show request_123" --first-log-line "upstream timeout" --severity normal
tovuk support resolve ticket_0123456789abcdef0123
tovuk abuse report https://example.com "Phishing page" "Credential collection form" --category phishing --reporter-email reporter@example.com --evidence "Screenshot URL and request id"
tovuk abuse list
tovuk abuse appeal abuse_0123456789abcdef0123 "Removed the reported public target." --evidence "remediation note"
tovuk abuse list --operator
tovuk abuse triage abuse_0123456789abcdef0123 "Reviewed reporter evidence."
tovuk abuse notify-owner abuse_0123456789abcdef0123 "Owner-visible notice recorded."
tovuk abuse quarantine abuse_0123456789abcdef0123 "Confirmed abuse and preserved evidence."
tovuk abuse resolve abuse_0123456789abcdef0123 "Reporter issue remediated."
tovuk abuse reject abuse_0123456789abcdef0123 "Evidence did not match the reported target."
tovuk abuse release abuse_0123456789abcdef0123 "Quarantine released after remediation."
```

Scraper Requests are for public data only. Do not send cookies, passwords,
account tokens, private session data, private account content, private
repository credentials, or proxy URLs.

The public CLI does not deploy websites, backends, databases, workers, storage
buckets, queues, cron jobs, custom domains, secrets, or other customer
infrastructure.
