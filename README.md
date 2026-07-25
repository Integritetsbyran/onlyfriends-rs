# OnlyFriends

## Release

Releases are fully automated via GitHub Actions. Pushing a signed, annotated tag triggers the release workflow.

### Requirements

- The tag and the commit it points to must both be signed.
- The signing GPG key must be included in the repository's `GPG_PUBLIC_KEYRING` secret.
- The tag version must match the version of `onlyfriends-relay` in `relay/Cargo.toml`.

### Creating a release

```bash
# Ensure the crate version matches the intended release
cargo metadata --no-deps --format-version=1 | jq '.packages[] | select(.name == "onlyfriends-relay").version'

# Create and push a signed tag
git tag -s v1.2.3 -m "v1.2.3"
git push origin v1.2.3
```

The CI workflow will then:

1. Verify the tag and commit signatures against the trusted keyring.
2. Confirm the tag version matches the crate version.
3. Build and push the Docker image to `ghcr.io/<org>/<repo>:relay-1.2.3`.
4. Create a GitHub release with the image reference.
