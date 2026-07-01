use super::constants::VERSION;

const HELP_BODY: &str = r#"
Usage:
  tovuk login [--token <token>] [--api <url>] [--json|--output json|text]
  tovuk account show [--api <url>] [--json]
  tovuk account activity [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk api-key list [--api <url>] [--json]
  tovuk api-key create "Production scraper" [--api <url>] [--json]
  tovuk api-key revoke <api_key_id> [--api <url>] [--json]
  tovuk pricing [--api <url>] [--json]
  tovuk scraper list [--api <url>] [--json]
  tovuk scraper health [--api <url>] [--json]
  tovuk scraper show <scraper> [--api <url>] [--json]
  tovuk request list [--limit <n>] [--api <url>] [--json]
  tovuk request create <scraper> '{"query":"coffee shops","limit":100}' [--limit <n>] [--api <url>] [--json]
  tovuk request show <request_id> [--api <url>] [--json]
  tovuk request results <request_id> [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk request cancel <request_id> [--api <url>] [--json]
  tovuk usage [--api <url>] [--json]
  tovuk billing [checkout plus|checkout pro|checkout max|portal] [reason] [--api <url>] [--json]
  tovuk support list [--limit <n>] [--api <url>] [--json]
  tovuk support create "Subject" "Details" [--request-id <request_id>] [--scraper-id <scraper>] [--failing-command <command>] [--first-log-line <line>] [--severity low|normal|urgent] [--api <url>] [--json]
  tovuk support resolve <ticket_id> [--api <url>] [--json]

Agent contract:
  - Set TOVUK_OUTPUT=json once for agent sessions, or pass --json per command. Use --output text to force human-readable output for one command.
  - Use tovuk scraper list --json to choose a public-data scraper, tovuk scraper health --json to inspect managed reader and proxy readiness, tovuk request create <scraper> '<json>' --json to create paid scraper work, tovuk request show <request_id> --json to poll status, and tovuk request results <request_id> --json to fetch stored records.
  - Scraper requests must use public URLs, public search terms, public profile handles, or public place ids only. Do not send cookies, passwords, account tokens, private session data, proxy URLs, or private account content.
  - Use tovuk pricing --json and tovuk usage --json before high-count scraper requests. Inspect priceEvents[].usdMicros, billingEstimate.lineItems, and account balance before creating large jobs.
  - Use tovuk api-key create "Production scraper" --json for scripts, save the returned token immediately, and revoke old keys with tovuk api-key revoke <api_key_id> --json.
  - When a plan limit blocks work, choose a plan and run tovuk billing checkout plus --json, tovuk billing checkout pro --json, or tovuk billing checkout max --json, then show the returned URL to the human.
  - For invoices, payment methods, or subscription changes, run tovuk billing portal and show the returned URL to the human.
  - Create support tickets with command output, request id when available, and the first actionable error line; AI/API agents can also call POST /v1/support/tickets with an account API key.
  - Resolve support tickets after the issue is fixed so later agents do not duplicate work.
  - This public CLI does not deploy websites, backends, databases, workers, storage buckets, queues, cron jobs, custom domains, secrets, or any other customer cloud resource.
"#;

pub(crate) fn help_text() -> String {
    format!("Tovuk {VERSION}\n{HELP_BODY}")
}
