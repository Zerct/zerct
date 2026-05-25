# npm Support Request: Allow Unscoped `zerct`

Opened as GitHub Community npm discussion:
https://github.com/orgs/community/discussions/196901

Subject: Please allow publishing unscoped package `zerct`

Hello npm support,

I am requesting review/approval to publish the unscoped npm package name
`zerct`.

The publish attempt was rejected with:

```text
403 Forbidden - PUT https://registry.npmjs.org/zerct - Package name too similar
to existing package react; try renaming your package to '@burakbayir/zerct' and
publishing with 'npm publish --access=public' instead
```

This package is for Zerct, a Rust backend hosting platform. The name is our
brand and is not intended to imitate React:

- Company/product name: Zerct
- Website: https://zerct.com
- App domain: https://zerct.app
- GitHub organization: https://github.com/Zerct
- Public source repository: https://github.com/Zerct/zerct
- npm organization/scope: @zerct
- X profile: https://x.com/zerctcloud
- Requested package: `zerct`
- Temporary scoped package: `@zerct/zerct`
- npm account/org owner: burakbayir

The package is a CLI for deploying Rust backends to Zerct. The intended command
is:

```sh
npx zerct deploy
```

The package metadata and source are public in the repository above. The package
does not reference React, does not depend on React, and is not related to the
React ecosystem.

Could you please review and allow the unscoped package name `zerct` for this
brand?

Thank you.
