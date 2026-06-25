# tovuk

Python package for launching the native Tovuk Rust CLI. It installs or
downloads the same binary used by npm, Homebrew, and Cargo.

```sh
pipx install tovuk
tovuk login --json
tovuk account show --json
tovuk pricing --json
tovuk scraper list --json
tovuk scraper health --json
tovuk scraper show tiktok --json
tovuk request list --limit 20 --json
tovuk request create reddit '{"subreddit":"rust","sort":"new","limit":100}' --json
tovuk request create tiktok '{"operation":"search","query":"rust programming","limit":100}' --json
tovuk request show request_123 --json
tovuk request results request_123 --limit 1000 --json
tovuk usage --json
tovuk billing checkout --json
tovuk billing portal --json
```

Set `TOVUK_OUTPUT=json` for agent sessions. Set
`TOVUK_NATIVE_BINARY=/path/to/tovuk` to test a local binary.

The CLI does not deploy websites, backends, databases, workers, storage buckets,
queues, cron jobs, custom domains, secrets, or other customer infrastructure.

Scraper requests are public data only. Do not send cookies, passwords, account
tokens, private session data, private account content, private repository
credentials, or proxy URLs.

Homebrew uses the main public repository tap:

```sh
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
```
