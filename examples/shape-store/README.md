# Tovuk Shape Store

A no-admin fullstack ecommerce example for Tovuk.

The storefront mirrors the sparse product-grid and in-place product-detail flow
of the current Yeezy storefront, but all product visuals are local black SVG
shape assets. No Yeezy branding or product imagery is copied.

## Run Locally

Start the Rust API:

```sh
cargo run --manifest-path api/Cargo.toml
```

Start the Vite frontend:

```sh
npm --prefix web run dev -- --host 127.0.0.1 --port 5174 --strictPort
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
