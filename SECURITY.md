# Security Policy

## Supported Versions

Security fixes target the latest published Tovuk release. Upgrade before
reporting a problem that is already fixed in a newer release.

## Report A Vulnerability

Use [GitHub private vulnerability reporting](https://github.com/tovuk/tovuk/security/advisories/new).
If that channel is unavailable, email `support@tovuk.com` and ask for a private
security-reporting channel. Do not open a public issue for an undisclosed
vulnerability.

Include the affected version, impact, reproduction steps, and any suggested
mitigation. Remove credentials, customer data, and unrelated private material
from the report.

## Scope

This repository covers the native CLI, npm and PyPI launchers, Homebrew
formula, release workflows, public documentation, and public API contract. The
hosted Tovuk service is not implemented here, but service vulnerabilities can
be reported through the same private channel.

## CI Trust Boundary

The organization ruleset must bind the `pull_request` history audit to an exact
reviewed workflow commit. The audit has read-only permissions, checks out only
that workflow source, and treats pull-request objects as data. Ordinary
pull-request and push runs are defense in depth because their workflow source
can come from the event commit. Release-tag rules must restrict creation to the
reviewed release automation. Local pre-push checks remain mandatory; no
ordinary event workflow is described as an exhaustive enforcement boundary.

Coordinate public disclosure through the private report so users have a
reasonable opportunity to update.
