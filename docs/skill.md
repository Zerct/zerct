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
manage billing.

```sh
export TOVUK_OUTPUT=json
tovuk account show
tovuk pricing
tovuk usage
tovuk scraper list
tovuk scraper health
tovuk scraper show tiktok
tovuk request create tiktok '{"operation":"search","query":"rust programming","limit":100}'
tovuk request create github '{"operation":"codeSearch","query":"serde language:Rust","maxRepos":3,"limit":100}'
tovuk request create linkedin '{"operation":"post-search","query":"b2b sales","sort_type":"date_posted","author_company_urns":"1035","limit":50}'
tovuk request show request_123
tovuk request results request_123 --limit 1000
tovuk billing checkout
tovuk billing portal
```

Scraper requests are public data only. Do not send cookies, passwords, account
tokens, private session data, private account content, private repository
credentials, or proxy URLs.

The public CLI does not deploy websites, backends, databases, workers, storage
buckets, queues, cron jobs, custom domains, secrets, or other customer
infrastructure.
