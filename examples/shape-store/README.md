# Tovuk Shape Store

A no-admin fullstack ecommerce example for Tovuk.

The storefront mirrors the sparse product-grid, full-screen product overlay,
size-selection, and full-screen bag checkout flow of the current Yeezy storefront,
but all product visuals are generated from black shape assets. No Yeezy branding
or product imagery is copied.

## Run Locally

Inspect the local plan first:

```sh
npx -y tovuk@latest dev --json
```

This project pins stable local ports in `tovuk.toml`:

```toml
[dev]
worker_port = 3001
frontend_port = 5174
```

If the planned ports are available, start both local processes:

```sh
npx -y tovuk@latest dev --output text
```

Use `--worker-port <port>` or `--frontend-port <port>` for a one-run override
when another local app already owns the configured ports.
Tovuk strips `[dev]` from deploy payloads and source archives, so these local
ports do not affect production.

Manual fallback: start the Rust API on a free port:

```sh
PORT=3001 cargo run --manifest-path api/Cargo.toml
```

Then start the Vite frontend against that API:

```sh
VITE_API_URL=http://127.0.0.1:3001/api npm --prefix web run dev -- --host 127.0.0.1 --port 5174 --strictPort
```

Then open `http://127.0.0.1:5174/`.

## Check And Deploy

```sh
npx -y tovuk@latest check
npx -y tovuk@latest deploy . --wait --wait-timeout 600
```

## Product Media

Local development uses the SVG files in `web/public/products`.

The published `shape-store.tovuk.app` example uses generated PNG versions of
those same black shape assets from Tovuk Object Storage:

```sh
./scripts/upload-product-media.sh
```

The script renders the local SVG assets into `.tovuk/product-media`, uploads
them to public object storage under `products/`, and lists the uploaded objects.

Useful overrides:

```sh
TOVUK_GENERATE_ONLY=1 ./scripts/upload-product-media.sh
TOVUK_SERVICE=my-store ./scripts/upload-product-media.sh
TOVUK_PRODUCT_MEDIA_PREFIX=products ./scripts/upload-product-media.sh
VITE_PRODUCT_MEDIA_BASE_URL=https://media.tovuk.app/my-store/products npm --prefix web run build
```

This keeps the public example license-safe while still exercising Tovuk's
object-storage media path end to end.

## Checkout Modes

`POST /api/checkout` supports two modes:

- Without `STRIPE_SECRET_KEY`, it returns a demo receipt so the public example
  can be tested and deployed without private credentials.
- With Stripe configured, it creates a Stripe Checkout Session and redirects
  the browser to the returned Checkout URL.

To enable real Stripe Checkout after deploying:

```sh
npx -y tovuk@latest secrets set --service shape-store STRIPE_SECRET_KEY=sk_live_...
npx -y tovuk@latest secrets set --service shape-store PUBLIC_BASE_URL=https://shape-store.tovuk.app
```

Use Stripe test keys first when validating a new deployment.

## Product Catalog

The Rust API and TanStack frontend use `web/src/catalog.json` as the single
catalog source. Edit products there, then run the API and frontend checks before
deploying.
