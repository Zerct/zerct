# tovuk

Native Tovuk CLI distributed through npm. The package installs or downloads the
same Rust binary used by Cargo, Homebrew, and PyPI.

```sh
npm install -g tovuk
tovuk login --json
tovuk account show --json
tovuk api-key list --json
tovuk api-key create "Production scraper" --json
tovuk api-key revoke api_key_0123456789abcdef01234567 --json
tovuk pricing --json
tovuk scraper list --json
tovuk scraper health --json
tovuk scraper show tiktok --json
tovuk request create github '{"query":"mcp server","language":"Rust","limit":100}' --json
tovuk request create tiktok '{"operation":"search","query":"rust programming","limit":100}' --json
tovuk request show request_123 --json
tovuk request results request_123 --limit 1000 --json
tovuk usage --json
tovuk billing checkout --json
tovuk billing portal --json
tovuk support list --limit 20 --json
```

The npm package exposes `bin/tovuk` and has no runtime JavaScript
dependencies. Set `TOVUK_NATIVE_BINARY=/path/to/tovuk` to test a local binary.

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
