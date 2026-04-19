# Packaging

Instructions for publishing Anvil to platform-specific package managers.

## winget (Windows Package Manager)

### First-time submission

The `winget-releaser` automation requires at least one version to already exist in
[microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs). The first version must be submitted manually.

**1. Get the SHA-256 hash from the release**

```powershell
# Download the .sha256 file from the GitHub release
$tag = "v1.0.0"
$asset = "anvil-${tag}-x86_64-pc-windows-msvc.zip"
Invoke-WebRequest "https://github.com/kafkade/anvil/releases/download/${tag}/${asset}.sha256" -OutFile sha256.txt
Get-Content sha256.txt
```

**2. Update the manifest**

Edit `packaging/winget/kafkade.anvil.installer.yaml`:
- Replace `REPLACE_WITH_ACTUAL_SHA256_FROM_RELEASE` with the actual hash
- Verify the `InstallerUrl` matches the release URL
- Update `PackageVersion` in all three files if different from `1.0.0`

**3. Validate the manifest locally**

```powershell
winget validate packaging/winget/
```

**4. Test the install locally**

```powershell
winget install --manifest packaging/winget/
anvil --version
```

**5. Submit to winget-pkgs**

```powershell
# Fork microsoft/winget-pkgs, then:
# Copy manifests to: manifests/k/kafkade/anvil/1.0.0/
# Create a PR following https://github.com/microsoft/winget-pkgs/blob/master/CONTRIBUTING.md

# Or use wingetcreate CLI:
wingetcreate submit packaging/winget/
```

**6. Verify after merge**

```powershell
winget search anvil
winget install kafkade.anvil
anvil --version
```

### Automated updates (subsequent releases)

After the first version is accepted, the release workflow (`.github/workflows/release.yml`)
automatically submits updated manifests to winget-pkgs on each tagged release using
[winget-releaser](https://github.com/vedantmgoyal9/winget-releaser).

**Required secret**: `WINGET_TOKEN` — a classic GitHub PAT with `public_repo` scope.
Create one at https://github.com/settings/tokens and add it to the repo secrets.

The automation:
1. Triggers after the GitHub Release is created
2. Matches the Windows `.zip` installer from the release assets
3. Generates updated manifest files with the correct version and SHA-256
4. Creates a PR to `microsoft/winget-pkgs` automatically

### Manifest format

The `packaging/winget/` directory contains a multi-file manifest:

| File | Purpose |
|------|---------|
| `kafkade.anvil.yaml` | Version manifest (ties identifier to version) |
| `kafkade.anvil.installer.yaml` | Installer details (URL, hash, architecture, type) |
| `kafkade.anvil.locale.en-US.yaml` | Package metadata (description, tags, license) |

The installer type is `zip` with `NestedInstallerType: portable` — winget extracts the zip
and registers `anvil.exe` as a portable command.
