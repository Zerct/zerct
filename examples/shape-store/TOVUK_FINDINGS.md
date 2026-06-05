# Tovuk Ecommerce MVP Findings

Date: 2026-06-05

## Current result

- Built a no-admin `fullstack-rust-tanstack` ecommerce MVP in this directory.
- Deployed it to Tovuk production.
- Live URL: https://hello-service.tovuk.app
- Latest verified deploy: `deploy_53`, `job_54`, status `succeeded`, service runtime status `running`.
- Patched and released Tovuk CLI `0.1.87` during this pass to remove JSON-mode deploy progress noise.
- Added and released Tovuk CLI `0.1.88` during this pass to make local fullstack UX testing easier with `tovuk dev`.
- Added and released Tovuk CLI `0.1.89` during this pass to add a static Next.js frontend template, make generated frontend templates default to npm consistently, and exclude common frontend build outputs from deploy archives.
- Added and released Tovuk CLI `0.1.90` during this pass to make plain
  `tovuk deploy --dry-run` a compact human-readable preflight while preserving
  the full structured report behind `--json`.
- Added and released Tovuk CLI `0.1.91` during this pass to add
  `tovuk sqlite batch` for transactional migrations, seed data, and imports.
- Added and released Tovuk CLI `0.1.92` during this pass to add compact
  `tovuk service status <service> --json` for quick post-deploy live checks.
- Added and released Tovuk CLI `0.1.93` during this pass to remove the
  misleading `queued <job>` summary line after `tovuk deploy --wait` has
  already streamed a final build status.
- Patched and deployed the Tovuk engine router wake path so sleeping fullstack services can wake instead of returning the platform `503` routing fallback.
- Updated the ecommerce example product flow to match the current Yeezy
  interaction more closely: product clicks keep the URL at `/`, transition into
  a full-screen product rail with adjacent products offscreen, and the `+`
  opens the inline size selector in place.
- Updated the ecommerce example cart flow to match the same reference more
  closely: selecting size `9` adds `YS-02` at `$50`, keeps the product overlay
  open, and the bag opens a full-screen `YZY WALLET`-style order summary.

## What I built

- Rust Worker API routes:
  - `GET /api/healthz`
  - `GET /api/products`
  - `POST /api/orders`
- TanStack/Vite storefront with:
  - sparse product-code grid inspired by the current minimal visual pattern at https://yeezy.com/
  - no admin dashboard
  - no demo bar
- local SVG product assets using black-only shapes
  - 50 product-code items across Yeezy-like categories
  - category filters
  - name-only product grid
  - full-page Yeezy-like product detail state with back control, bag icon,
    carousel dots, price, plus button, and inline size selector
  - full-screen wallet/order-summary cart overlay
  - quantity controls
  - checkout form
  - order receipt

I did not copy Yeezy branding or product imagery. Product visuals are local black SVG shape assets as requested.

## Verification

Local checks:

- `npx -y tovuk@latest check`: passed
- `npx -y tovuk@latest check --json`: passed
- Latest `npx -y tovuk@latest check`: passed
- `npm run typecheck`: passed
- `npm run lint`: passed, including `No code duplication found` and no complexity failures
- `npm run build`: passed
- Rust fmt/check/test/clippy: passed through Tovuk check
- Frontend typecheck/lint: passed through Tovuk check
- Black-only SVG audit: 50 product SVGs, no non-black color tokens found
- Overlay visual audit: cart/product overlays no longer dim the grid, so black
  shapes still render black behind an open drawer

Production deploy and API checks:

- `TOVUK_OUTPUT=json npx -y tovuk@latest deploy --dry-run .`: passed
- Latest `npx -y tovuk@latest deploy --dry-run .` confirms SQLite is enabled
  and the service check passes.
- `npx -y tovuk@latest deploy . --wait --wait-timeout 600`: passed
- Latest `npx -y tovuk@latest deploy . --wait --wait-timeout 600`: passed
  for `deploy_45` / `job_46`, using public CLI `0.1.93`.
- Latest local CLI deploy passed for `deploy_53` / `job_54`, using local
  Tovuk CLI `0.1.95`.
- `curl https://hello-service.tovuk.app/api/healthz`: returned `{"ok":true}`
- `curl https://hello-service.tovuk.app/api/products`: returned 50 products
- `curl -X POST https://hello-service.tovuk.app/api/orders ...`: returned an order receipt
- Latest deploy logs show `Checks passed`, Vite build output, and `Deploy promoted.`
  Production `https://shape-store.tovuk.app/api/healthz` returned `{"ok":true}`,
  and `https://shape-store.tovuk.app/api/products` returned 50 products with
  `YS-02` priced at `5000` cents.

Browser and UX checks:

- Visited https://yeezy.com/ in the in-app Browser for reference:
  - top category labels: `NEW`, `MENS`, `WOMENS`, `FOOTWEAR`, `ACCESSORIES`, `SLIDES`
  - product-code naming pattern such as `YS-02`, `SL-03`, `SG-03`, `SL-01`
  - compact product grid uses product images and product-code labels without visible prices on the grid
  - product click keeps the URL at `/`, transforms into a full-page product
    detail state with a back arrow and bag icon, then `+` opens an inline
    `SELECT SIZE` panel
- Opened the local storefront in the in-app Browser.
- Opened the production storefront in the in-app Browser.
- Confirmed production page had no `API FALLBACK` state.
- Confirmed production deploy `deploy_37` and `deploy_41` in Browser and
  Tovuk CLI status output.
- Confirmed production grid, product detail, size picker, cart, and receipt
  states through Browser or Playwright fallback.
- Latest local Browser check on `http://127.0.0.1:5175/` with
  `VITE_API_URL=http://127.0.0.1:3001/api` confirmed:
  - home grid uses the Yeezy-like sparse category/product layout
  - product click keeps the URL at `/`
  - focused product view shows back, bag, large black shape, carousel dots,
    product code, price, and plus
  - plus opens the Yeezy-like `? / SELECT SIZE / X` size selector with sizes
    `4` through `16`
  - selecting size `9` increments the bag count while staying in the product
    detail state
  - bag opens a full-screen order summary with `YZY WALLET`, product thumbnail,
    size, quantity controls, `$50` subtotal/total, `YZY CODE`, and express
    checkout buttons
  - no horizontal overflow at `429px` viewport width
- Confirmed responsive Browser screenshots at:
  - mobile `390x844`
  - tablet `768x1024`
  - desktop `1280x800`
- Confirmed no horizontal overflow in Yeezy or the ecommerce example at those
  viewport sizes.
- Confirmed product detail, size picker, and cart states have no horizontal
  overflow at mobile, tablet, and desktop sizes.
- Confirmed latest production grid metrics on `deploy_41`:
  - `50` product tiles in `NEW`
  - `3` columns at `390x844`
  - `6` columns at `768x1024` and `1280x800`
  - no `API FALLBACK`
- Confirmed production Browser flow on `deploy_53`:
  - product detail shows `YS-02` at `$50`
  - selecting size `9` increments the bag count to `1`
  - cart opens as a full-screen `YZY WALLET` order summary
  - cart subtotal and total are `$50`
  - no horizontal overflow at `429px`
- Confirmed mobile viewport at `390x844`:
  - `50` product tiles
  - `3` columns
  - no horizontal overflow
  - no product-label overflow in the first visible product set
- Added product to cart through Browser DOM controls.
- Browser form typing was blocked by the in-app Browser virtual clipboard, so I used the bundled Playwright fallback for the text-entry part of checkout.
- Submitted checkout locally and on production with fake demo data.
- Confirmed receipt state in production:
  - `ORDER TOV-1780661758 RESERVED FOR $44.00`
- Confirmed latest receipt state in production after `deploy_43`:
  - `ORDER TOV-1780663538 RESERVED FOR $44.00`
- Captured desktop screenshot:
  - `output/browser/ecommerce-local-desktop.png`
  - `output/browser/yeezy-browser-reference.png`
  - `output/browser/ecommerce-live-before.png`
  - `output/browser/ecommerce-local-updated.png`
  - `output/playwright/ecommerce-local-updated-checkout.png`
  - `output/playwright/ecommerce-production-updated-checkout.png`
  - `output/playwright/ecommerce-production-deploy25-grid.png`
  - `output/playwright/ecommerce-production-deploy25-cart.png`
  - `output/playwright/ecommerce-production-deploy25-receipt.png`
  - `output/playwright/ecommerce-browser-deploy31-receipt-only.png`
  - `output/playwright/ecommerce-browser-deploy33-dense.png`
  - `output/browser/ecommerce-production-deploy37-home.png`
  - `output/browser/ecommerce-production-deploy37-detail.png`
  - `output/browser/ecommerce-production-deploy37-size.png`
  - `output/browser/ecommerce-production-deploy37-cart.png`
  - `output/playwright/ecommerce-production-deploy37-receipt.png`
  - `output/browser/responsive/yeezy-mobile-390x844.png`
  - `output/browser/responsive/ecommerce-mobile-390x844.png`
  - `output/browser/responsive/yeezy-tablet-768x1024.png`
  - `output/browser/responsive/ecommerce-tablet-768x1024.png`
  - `output/browser/responsive/yeezy-desktop-1280x800.png`
  - `output/browser/responsive/ecommerce-desktop-1280x800.png`
  - `output/playwright/ecommerce-browser-deploy35-dense.png`
  - `output/playwright/ecommerce-browser-deploy35-density-large.png`
  - `output/playwright/ecommerce-browser-deploy35-mobile.png`
  - `output/playwright/ecommerce-browser-deploy35-receipt.png`
  - `output/browser/latest/ecommerce-local-home-transition-final.png`
  - `output/browser/latest/ecommerce-local-detail-transition-final-no-footer.png`
  - `output/browser/latest/ecommerce-local-size-transition-final-no-footer.png`
  - `output/browser/latest/ecommerce-production-deploy43-home.png`
  - `output/browser/latest/ecommerce-production-deploy43-detail.png`
  - `output/browser/latest/ecommerce-production-deploy43-size.png`
  - `output/playwright/latest/ecommerce-production-deploy43-receipt.png`

## Tovuk issues found and fixed

### 1. Global install confusion

Initial command:

```sh
npm install tovuk
tovuk new hello-service --template fullstack-rust-tanstack
```

Result:

```text
zsh: command not found: tovuk
```

Agent/user impact:

- This is the first path many users try.
- It is unclear whether `tovuk` should be installed globally, invoked through `npx`, or installed from a different package.

Recommendation:

- Make docs and CLI examples lead with `npx tovuk@latest ...`.
- If global install is intended, make `npm install -g tovuk` explicit.

### 2. Production login CSP blocked OAuth

Observed errors:

```text
form-action 'self'
form-action 'self' https://api.tovuk.com
```

Root causes:

- Web login needed to post to `https://api.tovuk.com`.
- OAuth provider redirects also needed `https://github.com` and `https://accounts.google.com`.

Fixes deployed:

- `b7a6ee2 Allow API form posts for web login`
- `af335d9 Allow OAuth provider form posts for login`

Verification:

- `https://tovuk.com/login` CSP now includes API, GitHub, and Google form targets.
- Login worked after deployment.

### 3. SQLite usage metering locked during deploy logs

Failure:

```text
database is locked
```

Root causes:

- Usage metering opened deferred SQLite write transactions and could race on write upgrade.
- Build stdout/stderr were streamed concurrently and each line could meter a log event concurrently.

Fixes deployed:

- `27f5d54 Reserve SQLite write lock for usage metering`
- `3e74468 Serialize build log metering writes`

Verification:

- Full engine check passed.
- Subsequent ecommerce deploys progressed beyond log metering.

### 4. Engine deploy failed on noexec `/tmp`

Failure:

```text
rustup-init could not execute from /tmp
```

Root cause:

- Production hardening has `/tmp` mounted noexec.
- Rustup installer was using a temporary path under `/tmp`.

Fix deployed:

- `a6518cd Use executable temp dir for rustup installs`

Verification:

- Engine deploys now complete rustup installation/update through an executable temp directory.

### 5. Fullstack runtime artifact scanned frontend `node_modules`

Failure:

```text
artifact source contains unsupported filesystem entry:
/var/lib/tovuk-engine/builds/job_8/web/node_modules/.bin/tsgolint
```

Root cause:

- Fullstack runtime artifact validation scanned the whole build workspace after frontend checks installed dependencies.
- `node_modules/.bin` contains symlinks, which artifact validation rejected.

Fix deployed:

- `3ceb3ac Ignore generated frontend deps in runtime artifacts`

Verification:

- Regression test covers symlinks under `node_modules`.
- Deploy progressed beyond this failure.

### 6. Fullstack runtime artifact included Cargo intermediates

Failure:

```text
Rust Worker artifact is 5389190 bytes after compression,
above the 3 MiB plan limit
```

Root cause:

- Artifact packaging included Cargo release intermediates from `target/release/deps` after checks ran.
- The final runtime binary was small enough; generated intermediates pushed the artifact over the free-plan limit.

Fix deployed:

- `4e57c4b Exclude Cargo intermediates from runtime artifacts`

Verification:

- Regression tests cover ignoring Cargo intermediates and still counting the final release binary.
- Deploy progressed beyond artifact size validation.

### 7. Runtime token upsert schema mismatch

Failure:

```text
ON CONFLICT clause does not match any PRIMARY KEY or UNIQUE constraint
```

Root cause:

- Runtime token rotation used `on conflict (app_id, slot)`.
- Schema only had a non-unique index on `(app_id, slot)`.

Fix deployed:

- `c3f8405 Add runtime token slot uniqueness`

Verification:

- Production schema now has `app_runtime_tokens_app_slot_unique_idx`.
- Regression test applies the schema and executes the same upsert target.
- Final ecommerce deploy succeeded.

### 8. JSON deploy output leaked human progress text

Failure/friction:

```text
build job_20 queued
build job_20 running
{ ...json... }
build job_20 succeeded
```

Root cause:

- `progress()` printed human progress messages to stderr even when `TOVUK_OUTPUT=json` was set.
- That is normal for many CLIs, but bad for Codex-style agents because terminal output is usually consumed as one combined stream.

Fix released:

- `75aae52 Release Tovuk CLI 0.1.87`

Verification:

- Public repo `./scripts/check-all.sh`: passed.
- Release workflows succeeded: CI, npm, PyPI, crates.io, native binaries.
- `TOVUK_OUTPUT=json npx -y tovuk@latest deploy . --wait --wait-timeout 600` returned one parseable JSON object with no progress text.
- `npm view tovuk version`: `0.1.87`
- `npx -y tovuk@latest --version`: `0.1.87`
- PyPI lists release `0.1.87`; crates.io is visible through `cargo search tovuk` as `0.1.87`.

### 9. Local fullstack dev required manual command guessing

Failure/friction:

```sh
PORT=3000 cargo run --release
VITE_API_URL=http://127.0.0.1:3000/api bun run dev --host 127.0.0.1 --port 5174
```

Root cause:

- There was no `tovuk dev` command.
- A user or agent had to infer worker root, frontend root, worker port, frontend port, and the frontend API env variable from `tovuk.toml`.
- Vite could silently move from the planned port to another port when the default was occupied, making the printed/expected URL stale.

Fix released:

- Added `tovuk dev [path]`.
- `tovuk dev --json` returns a machine-readable local dev plan without starting child processes.
- `tovuk dev --output text` starts the local worker and frontend.
- Fullstack dev plans now wire:
  - worker: `PORT=<worker_port> cargo run --release`
  - frontend: `VITE_API_URL=http://127.0.0.1:<worker_port>/api <package-manager> run dev --host 127.0.0.1 --port 5173 --strictPort`
- `--strictPort` avoids stale frontend URLs when another process already owns the expected port.

Verification:

- Public repo `./scripts/check-all.sh`: passed.
- Local ecommerce plan returned the expected worker/frontend commands and env.
- With port `5173` occupied by the Tovuk engine web dev server, `tovuk dev --output text` failed clearly with `Port 5173 is already in use` instead of silently moving the frontend URL.
- Release workflows succeeded: CI, npm, PyPI, crates.io, native binaries.
- `npm view tovuk version`: `0.1.88`
- `npx -y tovuk@latest --version`: `0.1.88`
- `cargo search tovuk`: `0.1.88`
- PyPI direct release metadata exists for `0.1.88`.
- `TOVUK_OUTPUT=json npx -y tovuk@latest dev .` returns the ecommerce fullstack dev plan.

### 10. Static Next.js support was missing

Friction:

- Cloudflare Pages documents static Next.js as a normal framework path: use
  `next build`, set `output: "export"`, and deploy the generated `out`
  directory.
- Tovuk had static frontend support but no first-class static Next.js starter.
- Agents had to know Next static export details, output directory, package
  scripts, Tovuk's frontend-only JS/TS rule, and the right `tovuk.toml` shape.

Fix released:

- Added `tovuk new <path> --template next-static-frontend`.
- The template creates:
  - `next.config.mjs` with `output: "export"`
  - `[build].output = "out"`
  - strict TypeScript config with Next-required compiler settings
  - npm scripts for `dev`, `typecheck`, `lint`, and `build`
  - Oxlint type-aware checks plus Fallow dead-code, duplicate-code, and health
    gates
  - no Next API routes, middleware, SSR handlers, or server code
- `tovuk dev --json` now detects Next static frontends and returns:
  - `npm run dev -- --hostname 127.0.0.1 --port 5173`
  - `NEXT_PUBLIC_API_URL` for fullstack Next frontends

Verification:

- Fresh scaffold passed:
  - `npm install`
  - `tovuk check --json`
  - `npm run build`
  - `tovuk dev --json`
- The build generated `out/index.html`.
- `tsconfig.json` no longer mutates on first `next build`.
- Public repo `./scripts/check-all.sh`: passed.
- Release workflows succeeded: CI, npm, PyPI, crates.io, native binaries, docs.
- `npm view tovuk version`: `0.1.89`
- `npx -y tovuk@latest --version`: `0.1.89`
- `cargo search tovuk`: `0.1.89`
- PyPI and crates.io direct release metadata exist for `0.1.89`.
- GitHub release `v0.1.89` has native assets for macOS arm64, macOS x64, Linux x64, and Windows x64.
- `npx -y tovuk@latest new <tmp>/next-web --template next-static-frontend` created the static Next.js template with `[build].output = "out"`.

Honest caveat:

- A fresh `npm install` still reports a moderate npm audit warning for
  Next's transitive pinned `postcss@8.4.31`. A package override made npm mark
  the dependency tree invalid, so I removed it. This is upstream framework
  dependency friction, not a Tovuk check failure.

### 11. Generated frontend templates had package-manager mismatch

Failure/friction:

- The generated TanStack/fullstack frontend config preferred Bun.
- The scaffold message said `bun install or npm install`.
- If a user chose npm, the generated `tovuk.toml` still used `bun ci` and
  `bun run build`, so checks failed later.
- The TanStack starter also lacked a `dev` script, even though `tovuk dev`
  expected one.

Fix released:

- Generated frontend templates now default to npm commands:
  - `npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint`
  - `npm run build`
- The scaffold message now gives one exact next step: run `npm install`.
- The TanStack starter now includes `"dev": "vite"`.
- Bun remains supported when `bun.lock` exists, but the generated path is
  deterministic for humans and agents.

Verification:

- Public repo `./scripts/check-all.sh`: passed.
- Public contract checks passed for package versions, CLI contract, and docs.

### 12. Deploy archives included generated frontend output directories

Failure/friction:

- Local UX testing and static builds can create `.next`, `out`, `dist`,
  `build`, `.cache`, and `.turbo`.
- Deploy archives already excluded `node_modules` and Rust `target`, but not
  all common frontend build outputs.
- That can make an agent's perfectly reasonable "build before deploy" workflow
  accidentally upload generated artifacts or exceed archive size limits.

Fix released:

- Added these directories to deploy archive walk exclusions:
  - `.cache`
  - `.next`
  - `.turbo`
  - `build`
  - `coverage`
  - `dist`
  - `out`
  - `vendor`
- Added a regression test for common frontend generated output paths.

Verification:

- `cargo test --locked --release --all-targets --all-features`: passed.
- Public repo `./scripts/check-all.sh`: passed.

### 13. Router wake failed for sleeping fullstack services

Failure:

```text
GET https://hello-service.tovuk.app/api/healthz
503 {"ok":false,"message":"Tovuk runtime routing is temporarily unavailable."}
```

Root cause:

- The production service was deployed and static assets were served, but the
  runtime status was `sleeping`.
- Router wake used one SQLite statement with writable CTEs:
  update deployment slot, update app, insert wake request.
- The worker already used three simple statements inside a transaction for the
  same operation. The router-only writable CTE path failed and returned the
  generic runtime routing fallback.

Fix deployed:

- `d76590f Fix router wake request transaction`
- The router now uses the same atomic transaction shape as the worker:
  update active slot, update app, insert pending wake request, commit.
- Added a regression test that applies the real control-plane schema and proves
  duplicate wake requests leave one pending wake row while marking both app and
  slot `starting`.

Verification:

- `cargo test -p tovuk-router`: passed.
- `./scripts/check-rust-quality.sh`: passed.
- `./scripts/deploy-origin.sh engine`: passed and restarted `tovuk-api`,
  `tovuk-router`, and `tovuk-worker`.
- After production deploy, six retries of
  `curl https://hello-service.tovuk.app/api/healthz` returned `200 {"ok":true}`.
- The updated ecommerce production checkout also reached the Rust API and
  returned a receipt.

### 14. This example still used Bun after templates moved to npm

Friction:

- Tovuk CLI `0.1.89` made new generated frontend templates default to npm.
- This existing ecommerce example still had `bun.lock` and Bun commands in
  `tovuk.toml`.
- `tovuk check` passed, but it routed through `bun run typecheck` and
  `bun run lint`, making the example inconsistent with the lower-friction
  default path we now recommend.

Fix applied to the example:

- Replaced frontend check/build commands with:
  - `npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint`
  - `npm run build`
- Removed `web/bun.lock`.
- Added `web/package-lock.json`.

Verification:

- `npx -y tovuk@latest check`: now reports `npm run typecheck` and
  `npm run lint`.
- `npx -y tovuk@latest check --json`: config now shows npm commands.
- Deploy `job_24` used npm in production and succeeded.

### 15. Agent/browser tooling friction during form testing

Friction:

- The in-app Browser was useful for reference inspection, DOM snapshots,
  product clicks, and layout measurement.
- Browser screenshot capture started timing out after one tab got stuck.
- Browser text entry failed with:

```text
Browser Use virtual clipboard is not installed
```

Impact:

- This is not a Tovuk runtime bug, but it matters for AI-agent usability because
  checkout testing requires text entry.
- I had to switch to the bundled Playwright CLI fallback for the form-fill part
  of the local and production checkout flows.

Recommendation:

- Keep Browser for visual/DOM inspection when requested.
- For agent-grade ecommerce flow testing, prefer a deterministic Playwright
  command path for form fill, screenshots, and repeatable checkout scripts.
  Tovuk examples should include a short "test the deployed app" recipe that an
  agent can run without relying on a fragile browser clipboard surface.

### 16. Deploy dry-run default output was too noisy

Friction:

- `tovuk deploy --dry-run .` returned the same huge structured JSON as
  `--json`.
- In this ecommerce example that output was about 72 KB and included full
  billing line items, capability catalog, limits, config, checks, meter plan,
  usage, and next actions.
- That is useful for agents when explicitly requested, but it is a poor default
  for humans doing a quick preflight.

Fix released:

- Kept the full JSON report behind `--json`.
- Added a compact default text summary with:
  - dry-run status
  - workspace path
  - service name, kind, existence, and check status
  - enabled/disabled capabilities
  - active meters
  - billing and usage summary
  - the command to get full JSON
  - next actions

Verification:

- Public repo `./scripts/check-all.sh`: passed.
- GitHub Actions succeeded: CI, npm publish, PyPI publish, crates.io publish,
  and native binary release.
- `npm view tovuk version`: `0.1.90`.
- `npx -y tovuk@latest --version`: `0.1.90`.
- `cargo search tovuk`: `0.1.90`.
- PyPI direct release metadata exists for `0.1.90`.
- GitHub release `v0.1.90` has native assets for macOS arm64, macOS x64,
  Linux x64, and Windows x64.
- `npx -y tovuk@latest deploy --dry-run .` now prints the compact text
  summary.
- `npx -y tovuk@latest deploy --dry-run . --json` still returns the full
  machine-readable dry-run report.
- Patched local CLI output for this ecommerce app is now:

```text
dry_run ok
workspace /Users/burak/Developer/Tovuk/hello-service/.
service hello-service kind=fullstack exists=true check=passed
capabilities enabled=static_frontend,worker,logs,builds,usage_caps,billing,support,abuse disabled=sqlite,object_storage,kv,state,queue,cron,service_bindings,secrets,custom_domains
meters build_minutes,log_events,static_transfer_bytes,worker_cpu_ms,worker_requests,worker_transfer_bytes
billing plan=free estimated_monthly_total=$0.00 current_overage=$0.00
usage requests_day=60 build_minutes_month=2 log_events_month=758
json: tovuk deploy --dry-run --json
next: Review billingEstimate.lineItems and warnings.
next: Set hard usage caps for expected load.
next: Run `tovuk deploy --wait --json`.
```

### 17. Cart overlay made black product shapes look gray

Friction:

- The SVG assets were black-only, but the cart/product overlay scrim used a
  translucent white background.
- When the cart was open, the shapes behind it visually looked gray in
  screenshots, which violated the "all shapes must be black" requirement from a
  user perspective even though the underlying SVG files were black.

Fix applied to the example:

- Changed `.overlay-scrim` from a translucent white wash to `transparent`.
- The cart and product drawers still close on outside click, but they no longer
  recolor the product grid behind them.

Verification:

- `npm run typecheck`: passed.
- `npm run lint`: passed.
- `npm run build`: passed.
- `npx -y tovuk@latest check`: passed.
- `npx -y tovuk@latest deploy . --wait --wait-timeout 600`: deployed
  `deploy_25` / `job_26`.
- Production screenshot `output/playwright/ecommerce-production-deploy25-cart.png`
  shows black product shapes behind the open cart.
- Production checkout returned
  `ORDER TOV-1780650268 RESERVED FOR $29.00`.

### 18. Receipt state showed empty-cart UI after successful checkout

Friction:

- After checkout, clearing the cart made the drawer show receipt data mixed
  with empty-cart affordances.
- That looked like a failed order to a real user and confused the agent test
  because the UI contained both success and "NO ITEMS" signals.

Fix applied to the example:

- When a receipt exists and the cart is empty, the cart drawer now renders a
  receipt-only state.
- The drawer no longer shows `NO ITEMS`, `$0.00` totals, or the checkout form
  after a successful reservation.

Verification:

- Browser receipt screenshot:
  `output/playwright/ecommerce-browser-deploy31-receipt-only.png`.
- Latest production Browser checkout on `deploy_35` returned
  `ORDER TOV-1780659905 RESERVED $29.00`.
- Browser state confirmed:
  - `hasEmptyCartText: false`
  - `hasReserveForm: false`

### 19. GitHub Yeezy clone research could inform UI, but not code reuse

Research:

- https://github.com/malerba118/yeezy-store-clone
  - no license metadata
  - useful high-level pattern: fullscreen product grid, product-code labels,
    sparse controls, motion/detail focus
- https://github.com/dLuxKid/yeezy-clone-25
  - no license metadata
  - useful high-level pattern: hamburger on the left, separate `+` grid
    density control, compact header, cart on the right
- https://github.com/jehoonje/yeezy-eCommerce
  - no license metadata
  - useful high-level pattern: 9-column dense grid, `+` grid states, minimal
    transparent header
- https://github.com/simon-caceres/Yeezy-Shops
  - Apache-2.0
  - older Bootstrap/360-image shop, less relevant to the current Yeezy UI

Decision:

- I did not copy code or assets from the unlicensed repos.
- I used only the high-level UI patterns that are safe and obvious:
  - monospace, sparse visual system
  - left hamburger
  - separate `+` grid-density control
  - 9-column desktop dense grid
  - product-code-only grid tiles
- Product assets remain local black-only SVG shapes.

Verification:

- `NEW` now shows the full 32-product current drop instead of only the products
  explicitly tagged `NEW`.
- Latest production Browser stats on `deploy_35`:
  - `tileCount: 32`
  - `columns: 9`
  - `gridClass: product-grid density-dense`

### 20. SQLite multi-statement migrations were too hard for agents

Failure/friction:

- Tovuk's SQLite API supports a transactional `statements` array.
- The API error text tells users to split multiple SQL statements into that
  array.
- The public CLI only exposed `sqlite query`, so agents had to loop statements
  one command at a time for schema setup or seed data.

Fix released:

- Added `tovuk sqlite batch --service <service> DB '[{"sql":"select 1"}]' --json`.
- Released it as Tovuk CLI `0.1.91`.
- `sqlite batch` accepts either:
  - a JSON array of statement objects
  - an object with a `statements` array

Verification:

- Public repo checks passed:
  - `cargo fmt --manifest-path crates/tovuk/Cargo.toml --check`
  - `cargo test --manifest-path crates/tovuk/Cargo.toml`
  - `cargo clippy --manifest-path crates/tovuk/Cargo.toml --all-targets -- -D warnings`
  - `go run scripts/check-public-contracts/*.go package-versions`
  - `go run scripts/check-public-contracts/*.go cli-contract`
  - `go run scripts/check-public-contracts/*.go docs`
  - `./scripts/check-all.sh`
- Release checks passed:
  - `npm view tovuk version`: `0.1.91`
  - `npx -y tovuk@latest --version`: `0.1.91`
  - `cargo search tovuk --limit 1`: `tovuk = "0.1.91"`
  - PyPI direct metadata exists for `0.1.91`
  - GitHub release `v0.1.91` has macOS arm64, macOS x64, Linux x64, and
    Windows x64 native assets
- Live ecommerce DB batch smoke test passed:

```sh
npx -y tovuk@latest sqlite batch --service hello-service STORE_DB '[{"sql":"select count(*) as order_count, sum(amount_cents) as total_cents from orders"}]' --json
```

Result:

```json
{"order_count":3,"total_cents":11600}
```

### 21. SQLite resource capability and runtime usage need clearer guidance

Friction:

- I was able to create a SQLite resource while this example's local
  `tovuk.toml` still had `sqlite = false`.
- I fixed the example config to `sqlite = true`, but the original flow is
  still a platform friction because resource creation and declared local
  capabilities can drift.
- The runtime binding path is also not easy enough for a small MVP. Docs say
  deployed workers receive `TOVUK_API_BASE_URL`, `TOVUK_RUNTIME_TOKEN`,
  `TOVUK_SERVICE_ID`, and `TOVUK_SQLITE_DB`, then call Tovuk's API. In this raw
  TCP Rust starter, that still leaves the user or agent hand-rolling HTTP
  client behavior, auth headers, JSON, and error handling inside app code.

Impact:

- CLI-side SQLite is now usable for migrations and seed data.
- Worker-side SQLite writes are still too much work for an ecommerce example
  unless we add a small SDK/helper or a clearer template recipe.
- The checkout endpoint currently returns a real API receipt but does not
  persist the browser-created order into `STORE_DB`; the DB is exercised
  through CLI migration/query flows.

Recommendation:

- Add a small Rust helper crate or template module for runtime Tovuk API calls:
  `sqlite_query`, `sqlite_batch`, `kv_get`, `queue_send`, and typed error
  responses.
- Have `sqlite create` warn when the target service's latest local project
  config or deployed capability state does not have SQLite enabled.
- Add a fullstack example that persists orders into SQLite using the runtime
  token path once the helper exists.

### 22. Browser plugin docs and behavior mismatched during UX testing

Friction:

- The Browser documentation listed `networkidle` as a supported load state.
- The runtime rejected it:

```text
playwright_wait_for_load_state does not support networkidle
```

- Earlier in the same work, the Browser plugin cache path changed from
  `26.601.21317` to `26.602.30954`, so the agent had to reconnect through a
  different absolute plugin path.
- A Playwright locator click also timed out earlier on `View YS-02`; direct CUA
  coordinate clicks worked.

Impact:

- This is not a Tovuk app bug, but it matters because the user explicitly wants
  AI agents to be able to run the whole Tovuk flow.
- Browser automation needs either fully stable docs/runtime contracts or a
  fallback recipe that agents can use without losing momentum.

Recommendation:

- For Tovuk examples, include a deterministic Playwright smoke-test script for
  checkout flows and deployed-page screenshots.
- Keep Browser as the visual inspection tool, but do not rely on `networkidle`
  or clipboard-backed text entry as the only test path.

### 23. `service show --json` is complete but hard to skim programmatically

Friction:

- `tovuk service show hello-service --json` returns a rich report with top
  keys like `status`, `deploys`, `builds`, `resources`, `accountUsage`, and
  `billingEstimate`.
- For the common agent question "is the latest deploy live?", the answer lives
  under:
  - `status.service.runtime_status`
  - `status.latest_deploy.id`
  - `status.latest_build_job.status`
- I initially tried a plausible `.service` / `.latestDeployment` shape and got
  `null`, which is exactly the kind of slow-down agents hit when output shapes
  are large and nested.

Recommendation:

- Keep the current full report for `--json`.
- Add a compact machine-readable mode or documented jq recipe for common
  checks:

```sh
tovuk service show hello-service --json | jq '.status'
```

- The plain text `tovuk service show hello-service` already covers humans
  better than it did before.

Resolution:

- Released `tovuk service status <service> --json` in CLI `0.1.92`.
- Verified against production `hello-service` after deploys `deploy_37` and
  `deploy_41`.

### 24. GitHub Yeezy clone research was useful for behavior, not reusable code

Finding:

- GitHub search found two direct public clone candidates:
  - `malerba118/yeezy-store-clone`
  - `richieagama/yeezy-website-clone`
- Neither repository had a license file, so I did not copy code or assets.
- The useful lessons were behavioral:
  - sparse product-code grid
  - click-to-full-page product detail instead of a drawer
  - back arrow and bag icon stay pinned at the top
  - plus opens a size selector in place

Result:

- The example now implements those behaviors with local black SVG shape assets.

### 25. Browser is good for visual UX, but text entry is still blocked here

Friction:

- Browser successfully visited Yeezy and the deployed ecommerce site, clicked
  products, opened the size picker, selected sizes, opened the cart, and
  captured responsive screenshots.
- Browser `locator.fill` and `locator.type` both failed on checkout inputs:

```text
Browser Use virtual clipboard is not installed
```

Impact:

- Browser can cover visual/UI state transitions, but the final form-entry
  transaction still needed terminal Playwright in this environment.

Recommendation:

- For Tovuk example repos, keep deterministic Playwright smoke scripts for
  checkout and receipt flows so agents can finish transactions when Browser
  text entry is unavailable.
- Fix or document the Browser virtual clipboard requirement because it blocks
  realistic ecommerce checkout testing.

### 26. `tovuk deploy --wait` summary was misleading after a successful wait

Friction:

- After CLI `0.1.92`, a real deploy printed:

```text
build job_38 queued
build job_38 running
build job_38 succeeded
queued job_38
```

- The final `queued job_38` line reads like the deploy is still queued even
  though the wait already completed successfully.

Fix:

- Released CLI `0.1.93` so non-wait deploys still print `queued <job>`, but
  wait deploys do not repeat a stale queued summary after final build status.
- Verified with a real production deploy:

```text
build job_42 queued
build job_42 running
build job_42 succeeded
service service_7756bec4a1b0831a
url https://hello-service.tovuk.app
next tovuk logs --service service_7756bec4a1b0831a
```

- Verified the public `0.1.92` CLI still showed the stale line on ecommerce
  deploy `job_44`:

```text
build job_44 queued
build job_44 running
build job_44 succeeded
queued job_44
```

- Verified public CLI `0.1.93` through npm, `npx`, PyPI, and crates.io.
- Verified `npx -y tovuk@latest deploy . --wait --wait-timeout 600` no longer
  prints the stale queued summary:

```text
build job_46 queued
build job_46 running
build job_46 succeeded
service service_7756bec4a1b0831a
url https://hello-service.tovuk.app
next tovuk logs --service service_7756bec4a1b0831a
```

### 27. Responsive visual testing needs to be part of the default example loop

Finding:

- The Yeezy reference and the example both avoid horizontal overflow at:
  - mobile `390x844`
  - tablet `768x1024`
  - desktop `1280x800`
- The example's product grid, product detail, size picker, and cart states also
  avoid horizontal overflow at those sizes.

Friction:

- This required manual Browser scripting and ad hoc screenshot paths.

Recommendation:

- Add an example-owned visual smoke command that captures mobile/tablet/desktop
  screenshots after deploy.
- The smoke should test grid, product detail, size picker, cart, and receipt
  states.

### 28. Local frontend port collisions can silently test the wrong app

Friction:

- `http://127.0.0.1:5173/` was already occupied by the Tovuk marketing app:

```text
/Users/burak/Developer/Tovuk/engine/apps/web/node_modules/.bin/vite --host 127.0.0.1
```

- Browser and Playwright were then testing a valid page with title `Tovuk`,
  not the ecommerce example.

Impact:

- This is dangerous for users and AI agents because the page loads
  successfully, but it is the wrong app.
- Agent screenshots and locators can become misleading instead of failing fast.

Recommendation:

- Tovuk example docs and generated dev commands should prefer explicit
  `--strictPort` for frontend local testing.
- `tovuk dev` should print the actual frontend URL it bound, and should make
  port fallback impossible or very obvious.

Workaround used:

```sh
npm run dev -- --host 127.0.0.1 --port 5174 --strictPort
```

### 29. Browser screenshots are reliable after correct-port recovery, but error-page recovery is rough

Friction:

- Browser initially hit a localhost refusal and landed on Chrome's generated
  error page.
- Reusing that tab caused Browser URL-policy and screenshot/click timeouts.
- A fresh tab on the correct `5174` ecommerce URL fixed screenshot capture.

Impact:

- Agents may interpret Browser failures as app failures unless the workflow
  checks URL, title, and expected product buttons before screenshotting.

Recommendation:

- Example smoke scripts should start with explicit page identity checks:
  title, visible app heading, expected product locator, and current URL.
- Browser recovery should include "open a fresh tab on the intended local URL"
  when a tab is on a generated Chrome error page.

### 30. Checkout examples need a deployable no-secret mode

Finding:

- Stripe Checkout is the pragmatic payment path for this public ecommerce
  example.
- True Stripe Express Checkout Elements need publishable keys, HTTPS/domain
  setup, browser wallet eligibility, and a server-created Checkout Session.
- A public Tovuk example cannot require private Stripe keys just to complete a
  smoke test.

Fix applied to the example:

- Added `POST /api/checkout`.
- When `STRIPE_SECRET_KEY` is absent, the endpoint returns a `STRIPE DEMO`
  receipt so users and agents can exercise the whole checkout path.
- When `STRIPE_SECRET_KEY` and `PUBLIC_BASE_URL` are configured, the endpoint
  creates a Stripe Checkout Session and returns a redirect URL.
- Added a cart `EXPRESS CHECKOUT` button that redirects for real Stripe mode
  and shows a `STRIPE DEMO` receipt for demo mode.
- Declared `secrets = true` in `tovuk.toml`.

Verification:

- Local Browser flow on `http://127.0.0.1:5174/`:
  - product click stayed on `/`
  - `+` opened the Yeezy-like size selector
  - size `8` opened the cart
  - `EXPRESS CHECKOUT` returned a `STRIPE DEMO` receipt
  - no horizontal overflow at the current `429px` Browser viewport
- Direct checks passed:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - strict Rust clippy
  - `npx -y tovuk@latest check`

### 31. `secrets = true` maps to `tovuk env`, not `tovuk secrets`

Friction:

- The config capability is named `secrets`.
- The CLI command users need is `tovuk env set --service ... KEY=value`.
- `tovuk secrets --help` does not show a secrets subcommand; it falls back to
  the generic command list.

Impact:

- Users and agents naturally search for `tovuk secrets set` after reading
  `secrets = true`.
- For payment examples, this creates avoidable setup friction exactly where
  users are already handling sensitive keys.

Recommendation:

- Either add a `tovuk secrets` alias for `tovuk env`, or make docs and CLI help
  say clearly that capability `secrets` is managed with `tovuk env set`.

Fix released in Tovuk CLI `0.1.94`:

- Added `tovuk secrets list`.
- Added `tovuk secrets set --service <service> KEY=value`.
- Added `tovuk secrets put --service <service> KEY=value`.
- Added `tovuk secrets delete --service <service> KEY`.
- Kept `tovuk env` as the same API surface for existing scripts.

### 32. Icon-only controls need accessible labels for agent use

Finding:

- On the live Yeezy reference, the product-detail plus button and some top
  controls were visually obvious but not exposed with useful accessible names
  in Browser snapshots.
- I had to fall back to DOM node IDs for the Yeezy `+` click.

Fix applied to the example:

- The local example keeps visual icon-only controls, but exposes stable labels:
  - `Back to products`
  - `Open cart with <n> items`
  - `Select size for <product>`
  - `Close size picker`

Impact:

- Browser and AI agents can click the same visual controls through stable,
  human-readable locators.
- This also improves keyboard and screen-reader usability without changing the
  minimal UI.

### 33. Stripe dependencies can push Rust artifacts over free-plan limits

Failure:

```text
Rust Worker artifact is 3767844 bytes after compression,
above the 3 MiB plan limit
```

Root cause:

- Adding Stripe Checkout support through `reqwest` increased the Linux release
  artifact enough to exceed the free-plan runtime artifact limit.
- Local `tovuk check` and `deploy --dry-run` passed because the failure only
  appeared after the production release build and artifact validation.

Fix applied to the example:

- Added a release profile to the worker:
  - `strip = true`
  - `lto = "fat"`
  - `codegen-units = 1`
  - `panic = "abort"`
  - `opt-level = "z"`

Verification:

- Local release binary dropped from `3.9 MiB` to `1.5 MiB`.
- Local compressed binary dropped to `913 KiB`.
- Production deploy `deploy_49` / `job_50` succeeded.

Product recommendation:

- Tovuk should surface artifact-size risk earlier when dry-run can see a Rust
  worker with heavyweight dependencies, or add a `tovuk deploy --dry-run
  --build-artifact` mode that performs the release build and artifact
  validation without promotion.

### 34. Fresh-clone frontend checks failed before dependencies existed

Failure:

- Copying the ecommerce store into the public examples repo and running
  `tovuk check examples/shape-store` failed at the frontend lint step because a
  fresh clone had no `node_modules` directory yet.
- Pinning the example to Vite 8 also exposed a registry/cache mismatch where
  `npm ci` returned `ETARGET` even though the latest registry metadata listed
  that version.

Fix applied:

- Tovuk CLI `0.1.94` now runs the frontend dependency install step before
  frontend script checks when `node_modules` is missing.
- The public example pins Vite `7.2.7` and `@vitejs/plugin-react` `5.1.1`
  exactly so users and agents get deterministic installs.

Verification:

- Removed `node_modules` from the public example.
- `cargo run --manifest-path crates/tovuk/Cargo.toml -- check
  examples/shape-store` installed dependencies and passed backend checks,
  frontend typecheck, and frontend lint.

Product recommendation:

- Tovuk examples and templates should prefer exact, known-good frontend tooling
  versions when they are intended to be copied by agents.
- `tovuk check` should continue treating dependency installation as part of the
  preflight, not as hidden prerequisite knowledge.

### 35. Artifact-size dry-runs need an explicit build mode

Finding:

- Ordinary dry-run is correctly read-only, but after adding Stripe dependencies
  agents needed a way to validate worker compressed size before creating a
  production build.

Fix included in Tovuk CLI 0.1.95:

- Added `tovuk deploy --dry-run --build-artifact`.
- The command runs the configured local Rust worker release build without
  upload or promotion.
- It reports `artifactCheck.compressedBytes` and compares it with
  `limits.workerCompressedSizeMib`.

Verification:

- Ran `cargo run --manifest-path ../../crates/tovuk/Cargo.toml -- deploy
  --dry-run --build-artifact --json` from the public `examples/shape-store`.
- `artifactCheck.compressedBytes` was `935395` and the free-plan limit was
  `3145728`.
- The command returned `deployBehavior: local_build_no_upload_no_remote_build`
  and `ok: true`.

Caveat:

- This is a local-platform artifact check. The production Linux build remains
  authoritative, but this catches size risk earlier and gives agents a concrete
  remediation path.

### 36. `tovuk dev --json` needed port ownership status

Failure/friction:

- I opened `http://127.0.0.1:5173/` expecting the shape-store frontend, but an
  existing Tovuk app was already serving that port.
- `tovuk dev --json` showed the planned URLs and commands, but did not say
  whether those URLs were already occupied.

Fix included in Tovuk CLI 0.1.95:

- Added `dev.port_statuses` to the JSON dev plan.
- Each planned worker/frontend URL now reports `available`, `host`, `port`,
  `url`, and an `agent_instruction` when the port is already in use.
- The top-level `agent_instruction` warns agents to inspect
  `dev.port_statuses` before running `tovuk dev --output text`.

Verification:

- With local ports `3000` and `5173` occupied, `cargo run --manifest-path
  ../../crates/tovuk/Cargo.toml -- dev --json` reported both planned ports as
  unavailable.
- Added `port_status_reports_occupied_port` unit coverage.
- `cargo test --manifest-path crates/tovuk/Cargo.toml --locked --all-targets
  --all-features` passed with 54 tests.

## Remaining Tovuk friction

### High

- `--json` auth flows still need a more agent-readable shape. Agents need `login_url`, `user_code`, expiry, and current wait state as JSON before any long wait.
- Generated fullstack templates still make ordinary API work harder than necessary because the Rust worker is a raw TCP HTTP server. It is lightweight, but agents must hand-build routing, body parsing, CORS, and JSON handling.
- New static Next.js support is static-export only. That is correct for the current Tovuk runtime model, but users coming from Vercel will expect SSR and API routes unless docs and check errors keep saying "move server logic to Rust".

### Medium

- The new `tovuk service status <service> --json` covers compact live checks,
  but agent docs should consistently prefer it for post-deploy smoke tests and
  reserve `service show --json` for full inspection.
- `tovuk check --json` still prints both `run.health: /healthz` and `worker.health: /api/healthz` in config output for this fullstack app. It is harmless here, but confusing for agents deciding which health path matters.
- Browser login failures surface as browser console errors before they become Tovuk-branded user guidance.
- npm audit warnings from framework dependency trees are noisy for template users. Tovuk should decide whether template checks should surface audit guidance separately from source/lint/build checks.

## AI agent usability

Easy:

- `tovuk --help` and `tovuk check --json` are agent-friendly.
- The project config makes capabilities explicit.
- The final deploy loop is straightforward once auth and platform bugs are fixed.

Hard:

- Login and deploy failures required browser console inspection, deploy logs, engine code inspection, production schema checks, commits, pushes, and engine rollouts.
- Several failures happened only after earlier blockers were fixed, so an agent has to keep retrying the full production path.
- The platform should distinguish app failures from Tovuk platform failures more explicitly in deploy output.

## UX notes for the example

- Desktop layout is intentionally stark and sparse.
- Mobile layout uses three product columns and no horizontal scrolling.
- Tablet and desktop layouts use six product columns and no horizontal
  scrolling.
- Product grid shows product codes only; prices, size selection, and add-to-cart
  live in the full-page product detail state.
- Product detail now mirrors the current Yeezy flow more closely: the route
  does not change, the selected product is shown in a full-screen rail, and the
  inline size selector replaces the plus controls without showing the footer.
- Cart now mirrors the current Yeezy flow more closely: size selection adds to
  the bag without leaving product detail, and the bag opens a full-screen
  wallet/order-summary checkout overlay.
- Cart, quantity, and checkout flows work from the deployed site.
- I fixed the mobile cart trigger alignment after visual inspection.
- No demo label/bar is present.
