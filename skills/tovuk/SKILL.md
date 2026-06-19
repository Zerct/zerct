---
name: tovuk
description: Use Tovuk scraper APIs and manage Rust services with `tovuk`.
---

# Tovuk

Use when a user wants to create Tovuk scraper Requests, fetch stored Results,
deploy a Rust worker, static frontend, or full-stack service to Tovuk.

## Workflow

Set `TOVUK_OUTPUT=json` for agent sessions. Keep `--json` only when a single
command must force JSON without changing the session environment.

1. Ensure the project has `tovuk.toml`.
2. For Rust workers, ensure `Cargo.toml`, `Cargo.lock`, a health endpoint,
   `cargo fmt --all --check`, locked release-mode check/test/Clippy gates,
   strict Clippy resource lints, and small declared runtime resources.
3. For package static frontends, set `kind = "static_frontend"` and ensure
   `package.json`, TypeScript browser source, stable native type-aware
   typechecking, native linting, Fallow quality gates, a lockfile, and a strict
   frontend check command.
4. For plain static frontends without a package manager, set
   `kind = "static_frontend"`, require `index.html`, use
   `[build].check = ":"`, `[build].command = ":"`, and `[build].output = "."`.
5. For full-stack services, set `kind = "fullstack"` in one root
   `tovuk.toml`, configure `[worker].root` and `[frontend].root`, serve the
   frontend at `/`, and route API calls through same-origin `/api`.
6. Prefer the package manager that matches the committed lockfile. Generated
   templates default to npm; Bun remains supported when `bun.lock` is present.
   Use source-scoped Oxlint type-aware checks and Fallow for new package
   frontends. Avoid JavaScript-based lint, format, typecheck, dead-code, or
   duplicate-code tooling.
7. For a new full-stack project, run
   `tovuk new hello-service --template fullstack-rust-tanstack`.
8. Run `tovuk check`.
9. Run `tovuk check` when local tools are available.
10. Run `tovuk deploy --wait`.
11. If Tovuk returns an `agent_instruction`, apply it, rerun check, and
    redeploy.
12. If a build fails, run `tovuk logs --build <build_id>`, fix the
    first actionable log error, rerun check, and redeploy.

For scraper API work:

1. Run `tovuk scraper list --json`.
2. Run `tovuk scraper health --json` and inspect `scrapers[].status` before
   creating high-volume scraper Requests.
3. Run
   `tovuk request create <scraper> '{"query":"public search","limit":100}' --json`.
4. For GitHub, use public input only, for example
   `tovuk request create github '{"query":"mcp server","language":"Rust","limit":50}' --json`
   or
   `tovuk request create github '{"operation":"opportunities","query":"agent skills registry","limit":25}' --json`
   or
   `tovuk request create github '{"operation":"codeSearch","query":"serde language:Rust","maxRepos":3,"limit":25}' --json`
   or
   `tovuk request create github '{"operation":"codeSearch","query":"StreamableHTTPClientTransport","language":"TypeScript","repo":"modelcontextprotocol/typescript-sdk","path":"examples/client/src","limit":25}' --json`
   or
   `tovuk request create github '{"url":"https://github.com/rust-lang/rust/issues/1"}' --json`
   or
   `tovuk request create github '{"operation":"watchers","repo":"rust-lang/rust","limit":50}' --json`
   or
   `tovuk request create github '{"operation":"file","repo":"rust-lang/rust","path":"README.md","contentMaxChars":2000}' --json`
   or
   `tovuk request create github '{"operation":"trendingDevelopers","language":"rust","since":"weekly","limit":25}' --json`
   or
   `tovuk request create github '{"operation":"marketplace","searchQuery":"ci","limit":25}' --json`.
5. For LinkedIn, use public input only, for example
   `tovuk request create linkedin '{"operation":"post-search","query":"b2b sales","sort_type":"date_posted","author_company_urns":"1035","limit":25}' --json`
   or
   `tovuk request create linkedin '{"operation":"company-employees","identifier":"https://www.linkedin.com/company/google/","job_title":"engineer OR developer","max_employees":50}' --json`.
6. For X, use public input only, for example
   `tovuk request create x '{"query":"rust lang","product":"Latest","limit":100}' --json`
   or
   `tovuk request create x '{"url":"https://x.com/openai/status/1234567890","limit":1}' --json`.
7. Poll with `tovuk request show <request_id> --json`.
8. Fetch stored Records with `tovuk request results <request_id> --json`.
9. Do not send cookies, passwords, account tokens, GitHub tokens, private
   repository credentials, private session data, private account content, or
   proxy URLs. For GitHub, send only public repository queries, code search
   queries and filters, URLs, usernames, repository names, and public filters.
   For Instagram, send only public profile URLs, post URLs, reel
   URLs, hashtag URLs, usernames, shortcodes, media ids, hashtags, or search
   terms; Tovuk manages reader accounts internally. Tovuk manages X read
   accounts and managed proxy provider egress internally. For Reddit, send only public
   subreddit names, public subreddit URLs, search terms, post URLs, post ids,
   usernames, and public search filters such as `contentType` and
   `autoDiscoverSubreddits`, direct comment ordering via `commentSort`, and
   top-level output `fields`; Tovuk manages managed proxy provider egress internally.
   For LinkedIn, send only public job search URLs, job ids, company URLs or
   names, profile URLs or public identifiers, people-search filters, post URLs,
   post-search terms, content filters, public author/member/company/industry
   ids, and company employee filters; Tovuk manages managed proxy provider egress and reader
   sessions internally.
   For TikTok, send only public video URLs, profile URLs, usernames, video ids,
   hashtags, music ids, place ids, Ads Library URLs, Shop URLs, public Shop ids,
   public product ids, public search terms, and projection fields such as
   `fields` or `outputFields`. Public-input aliases such as `profileUrls`,
   `handles`, `queries`, `userSearch`, `videoSearch`, `sound`, `soundUrls`,
   `musicUrls`, `soundIds`, `region`, `startDate`, `endDate`, `minLikes`,
   `maxLikes`, `downloadVideos`, `downloadSubtitles`, and `transcribeVideos`
   are accepted; cookies, passwords, tokens, session data, and proxy URLs are
   not.

## Service resources

Agents can manage runtime resources without dashboard access:

```sh
tovuk service show <service> --json
tovuk scraper list --json
tovuk scraper health --json
tovuk request create google-maps '{"query":"coffee shops","limit":100}' --json
tovuk request create github '{"query":"mcp server","language":"Rust","limit":50}' --json
tovuk request create github '{"operation":"codeSearch","query":"serde language:Rust","maxRepos":3,"limit":25}' --json
tovuk request create github '{"operation":"codeSearch","query":"StreamableHTTPClientTransport","language":"TypeScript","repo":"modelcontextprotocol/typescript-sdk","path":"examples/client/src","limit":25}' --json
tovuk request create github '{"url":"https://github.com/rust-lang/rust/issues/1"}' --json
tovuk request create github '{"operation":"watchers","repo":"rust-lang/rust","limit":50}' --json
tovuk request create github '{"operation":"file","repo":"rust-lang/rust","path":"README.md","contentMaxChars":2000}' --json
tovuk request create github '{"operation":"trendingDevelopers","language":"rust","since":"weekly","limit":25}' --json
tovuk request create github '{"operation":"marketplace","searchQuery":"ci","limit":25}' --json
tovuk request create reddit '{"query":"rust lang","contentType":"both","autoDiscoverSubreddits":true,"maxSubreddits":5,"fields":["type","id","url","title","bodyText","score"],"maxResults":50}' --json
tovuk request create reddit '{"operation":"subreddit-profile","community":"rust"}' --json
tovuk request create linkedin '{"operation":"post-search","query":"b2b sales","sort_type":"date_posted","author_company_urns":"1035","limit":25}' --json
tovuk request create linkedin '{"operation":"company-employees","identifier":"https://www.linkedin.com/company/google/","job_title":"engineer OR developer","max_employees":50}' --json
tovuk request create tiktok '{"operation":"search","query":"rust programming","outputFields":["id","desc","author.uniqueId","stats.playCount"],"limit":30}' --json
tovuk request create tiktok '{"operation":"sound","soundUrls":["https://www.tiktok.com/music/original-sound-1234567890"],"limit":30}' --json
tovuk request create x '{"query":"rust lang","product":"Latest","limit":100}' --json
tovuk request show request_123 --json
tovuk request results request_123 --json
tovuk sqlite create --service <service> DB --json
tovuk kv create --service <service> CACHE --json
tovuk kv bulk put --service <service> CACHE '[{"key":"feature:search","value":"enabled"}]' --json
tovuk kv bulk get --service <service> CACHE feature:search user:1 --json
tovuk queue create --service <service> jobs --json
tovuk queue send --service <service> jobs '{"task":"sync"}' --json
tovuk queue send-batch --service <service> jobs '[{"body":{"task":"sync"}},{"body":{"task":"index"}}]' --json
tovuk queue metrics --service <service> jobs --json
tovuk cron create --service <service> nightly "0 0 * * *" --json
tovuk cron update --service <service> nightly "*/15 * * * *" --json
tovuk cron disable --service <service> nightly --json
tovuk state create --service <service> Room --json
tovuk binding create --service <service> AUTH_SERVICE --target auth-service --json
tovuk limits set worker_requests --period day --value 100000 --notify-at-percent 80 --json
tovuk billing checkout --json
tovuk billing portal
tovuk abuse report https://demo.tovuk.app "Phishing page" "Credential collection form" --category phishing --reporter-email reporter@example.com --evidence "Screenshot URL and request id" --json
tovuk abuse list --json
tovuk abuse appeal abuse_0123456789abcdef0123 "Removed the reported file and rotated credentials." --evidence "deploy_1 remediation log" --json
tovuk abuse list --operator --json
tovuk abuse triage abuse_0123456789abcdef0123 "Reviewed reporter evidence and target service metadata." --json
tovuk abuse notify-owner abuse_0123456789abcdef0123 "Owner-visible report recorded with evidence summary." --json
tovuk abuse quarantine abuse_0123456789abcdef0123 "Confirmed malware object and preserved scanner evidence." --json
tovuk abuse resolve abuse_0123456789abcdef0123 "Reporter issue remediated and clean deploy verified." --json
tovuk abuse reject abuse_0123456789abcdef0123 "Evidence did not match the reported target." --json
tovuk abuse release abuse_0123456789abcdef0123 "Owner removed object and redeployed clean build." --json
```

## Contract

Rust workers must listen on `0.0.0.0:$PORT`, expose the configured health
endpoint, pass `cargo fmt --all --check`, run locked release-mode `cargo check`
and `cargo test`, run strict all-target/all-feature Clippy with panic/unwrap
bans and resource-sensitive lints, and avoid direct `unsafe` in workspace
source.

Package static frontends must use `.ts` or `.tsx` browser source under `src`,
`app`, `pages`, `routes`, or `components`; install dependencies, run stable
native type-aware TypeScript checks, run native linting plus Fallow dead-code,
semantic duplicate-code, and health gates; build to `[build].output`; and
include `index.html`. Plain static frontends without `package.json` can use
`index.html`, `check = ":"`, `command = ":"`, and `output = "."`.

Full-stack frontends call same-origin `/api` for APIs and server-side logic.
JavaScript and TypeScript are frontend-only on Tovuk.

Abuse reports are API and CLI first. Create reports with target URL, category,
reporter email, and evidence. Service owners use `tovuk abuse list --json` and
`tovuk abuse appeal <report_id> --json` with remediation evidence. Operators use
`tovuk abuse list --operator --json`, then triage, notify-owner, quarantine,
resolve, reject, or release with preserved evidence.
