# Tovuk Shape Store

A no-admin fullstack ecommerce example for Tovuk.

The storefront mirrors the sparse product-grid, full-screen product overlay,
size-selection, and full-screen bag checkout flow of the current Yeezy storefront,
but all product visuals are local black SVG shape assets. No Yeezy branding or
product imagery is copied.

## Run Locally

Inspect the local plan first:

```sh
npx -y tovuk@latest dev --json
```

If the planned ports are available, start both local processes:

```sh
npx -y tovuk@latest dev --output text
```

If another local app already owns those ports, start the Rust API on a free port:

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

## Checkout Modes

`POST /api/checkout` supports two modes:

- Without `STRIPE_SECRET_KEY`, it returns a `STRIPE DEMO` receipt so the public
  example can be tested and deployed without private credentials.
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
