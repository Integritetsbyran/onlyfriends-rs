# OnlyFriends

## Creating a Release

Releases are automated via GitHub Actions.
To start the release workflow, push a signed tag in one of the following forms:
- `relay/vX.Y.Z`
- `desktop/vX.Y.Z`

**Note**:
- The tag and the commit it points to must both be signed.
- The signing GPG key must be included in the repository's `GPG_PUBLIC_KEYRING` secret.
- The tag version must match the version of corresponding crates `Cargo.toml`.

**Example**:
```bash
# Ensure the crate version matches the intended release
cargo metadata --no-deps --format-version=1 | jq '.packages[] | select(.name == "onlyfriends-relay").version'

# Create and push a signed tag
git tag -s "relay/v1.2.3" -m "relay/v1.2.3"
git push origin "relay/v1.2.3"
```

The CI workflow will then:

1. Verify the tag and commit signatures against the trusted keyring.
2. Confirm the tag version matches the crate version.
3. Build and push the package to GitHub.
4. Create a GitHub release and attach the package artifacts.
