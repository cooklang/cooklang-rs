# Publishing Instructions

This document explains how to publish the new `@cooklang/cooklang` package and deprecate the old `@cooklang/cooklang-ts` package.

## Overview

- **New package**: `@cooklang/cooklang` @ v0.17.2 (this directory)
- **Old package**: `@cooklang/cooklang-ts` @ v1.2.8 (in `../cooklang-ts/`)
- **Strategy**: Publish new package, then deprecate old one

## Prerequisites

1. You must have npm publish permissions for the `@cooklang` scope
2. You must be logged in to npm: `npm whoami`
3. If not logged in: `npm login`

## Step 1: Publish the New Package

From the `typescript/` directory in this repo:

```bash
# Make sure you're in the right directory
cd /Users/alexeydubovskoy/Cooklang/cooklang-rs/typescript

# Build the package
npm run build-wasm
npm run build

# Run tests to ensure everything works
npm test

# Verify the package contents
npm pack --dry-run

# Publish to npm (production release)
npm publish --access public

# Or for testing: publish to a different tag first
npm publish --access public --tag next
```

### Verify Publication

After publishing, verify the package:

```bash
# Check it's published
npm view @cooklang/cooklang

# Install it in a test project
mkdir /tmp/test-cooklang
cd /tmp/test-cooklang
npm init -y
npm install @cooklang/cooklang

# Test it works
node -e "const { Parser } = require('@cooklang/cooklang'); console.log(new Parser().parse('>> servings: 4'));"
```

## Step 2: Publish Deprecation Release for Old Package

From the `cooklang-ts/` directory:

```bash
# Switch to the old package directory
cd /Users/alexeydubovskoy/Cooklang/cooklang-ts

# Build the package (includes deprecation warning)
npm run build

# Publish version 1.2.8 with deprecation
npm publish
```

## Step 3: Officially Deprecate the Old Package

Use npm's deprecation feature:

```bash
npm deprecate @cooklang/cooklang-ts "This package is deprecated. Please use @cooklang/cooklang instead. Migration guide: https://github.com/cooklang/cooklang-rs/blob/main/typescript/MIGRATION.md"
```

This will:
- Show a warning when users run `npm install @cooklang/cooklang-ts`
- Display the deprecation message in the npm registry
- Not break existing installations

## Step 4: Update Documentation

1. **Update the main cooklang-rs README** if it references the old package
2. **Update any examples** in the repo to use `@cooklang/cooklang`
3. **Announce the change** in:
   - GitHub release notes
   - Cooklang website/docs
   - Discord/community channels
   - Twitter/social media

## Future Releases

For future releases of `@cooklang/cooklang`:

1. Update version in `Cargo.toml` (root crate)
2. Update version in `typescript/package.json` to match
3. Build and test: `npm run build-wasm && npm run build && npm test`
4. Publish: `npm publish --access public`

The version should always match the Rust crate version (e.g., `0.17.2`).

## Rollback Plan

If something goes wrong:

### Unpublish (within 72 hours of publishing)

```bash
npm unpublish @cooklang/cooklang@0.17.2
```

⚠️ **Warning**: Unpublishing is permanent and can break projects. Only do this for critical issues.

### Deprecate the New Version

```bash
npm deprecate @cooklang/cooklang@0.17.2 "This version has issues. Please use @cooklang/cooklang-ts@1.2.7 instead."
```

### Publish a Fix

```bash
# Fix the issue
# Bump version to 0.17.3
npm publish --access public
```

## Troubleshooting

### "You do not have permission to publish"

Make sure:
1. You're logged in: `npm whoami`
2. You have access to `@cooklang` scope
3. Contact the org owner to grant you access

### "Version already exists"

You can't republish the same version. Bump the version in `package.json` and try again.

### "Package size exceeds limit"

The WASM file is large (~2.8MB). This is expected. npm allows packages up to 10MB by default.

### Tests fail

Don't publish if tests fail. Fix the issues first:

```bash
npm test -- --reporter=verbose
```

## Post-Publication Checklist

- [ ] Verify package is visible on npmjs.com
- [ ] Test installation in a fresh project
- [ ] Update documentation/examples
- [ ] Announce the release
- [ ] Monitor for issues in the first 24-48 hours
- [ ] Respond to user questions/issues promptly

## Support Timeline

- **@cooklang/cooklang-ts v1.2.8**: Critical security fixes only
- **@cooklang/cooklang v0.17.2+**: Full support, active development

## Questions?

Open an issue on GitHub: https://github.com/cooklang/cooklang-rs/issues
