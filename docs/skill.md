---
name: zerct
description: Deploy Rust backends to Zerct and expose them as HTTPS APIs.
license: MIT
compatibility: Requires a Rust backend with Cargo.toml, Cargo.lock, zerct.toml, and a server that listens on 0.0.0.0:$PORT.
metadata:
  author: Zerct
  version: "0.1"
---

# Zerct

Use this skill when a user wants to deploy a Rust backend to Zerct, inspect a
deployment, read logs, or prepare a project for deployment.

## Project contract

Require these files before deploying:

- `Cargo.toml`
- `Cargo.lock`
- `zerct.toml`

The server must listen on `0.0.0.0:$PORT` and expose the configured health path.

## Commands

Check the project:

```sh
npx @zerct/zerct doctor
```

Deploy:

```sh
npx @zerct/zerct deploy
```

Use a persistent CLI:

```sh
brew tap Zerct/tap
brew install zerct
zerct deploy
```

## Agent behavior

When a Zerct command returns JSON with `agent_instruction`, follow that field
first, change the project, and retry the command. Do not guess around a failed
guardrail.
