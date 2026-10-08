# Passepartout

![Passepartout icon](data/icons/hicolor/scalable/apps/io.github.untoastedtoast.Passepartout.svg)

Place images on a canvas with padding, sized for Instagram. A GTK4 / libadwaita
app for Linux, written in Rust.

- Drag in images or whole folders, see a live preview, export in one click
- Canvas presets for current Instagram formats, default **1080 × 1440** (3:4, the profile grid preview)
- Padding, background color, output format, quality and output folder are all set in the UI
- Settings are remembered between runs

| Preset | Size |
| --- | --- |
| Profile grid 3:4 (default) | 1080 × 1440 |
| Feed portrait 4:5 | 1080 × 1350 |
| Feed square 1:1 | 1080 × 1080 |
| Feed landscape 1.91:1 | 1080 × 566 |
| Story / Reel 9:16 | 1080 × 1920 |

Any other width and height works too ("Custom"). Output files are named `<name>_canvas.<ext>`
and go to `~/Pictures/Passepartout` by default.

## Build and run

Needs a Rust toolchain plus the GTK 4 and libadwaita development files
(Fedora: `sudo dnf install gtk4-devel libadwaita-devel`).

```sh
cargo run -- [files or folders]
cargo test
```

## Flatpak

```sh
flatpak install flathub org.gnome.Sdk//50 org.gnome.Platform//50 org.freedesktop.Sdk.Extension.rust-stable//25.08
flatpak/generate-sources.sh   # needs uv; re-run after changing Cargo.lock
flatpak-builder --user --install --force-clean build-dir flatpak/io.github.untoastedtoast.Passepartout.yml
flatpak run io.github.untoastedtoast.Passepartout
```

## Install

Download a package for x86_64 from the [releases page](https://github.com/UntoastedToast/passepartout/releases), then:

| Package | Install |
| --- | --- |
| Flatpak (any distro) | `flatpak install --user passepartout-x86_64.flatpak` |
| `.deb` (Debian 13+, Ubuntu 24.04+) | `sudo apt install ./passepartout_*_amd64.deb` |
| `.rpm` (Fedora) | `sudo dnf install ./passepartout-*.x86_64.rpm` |

## Verify

Every package comes with a signed build provenance attestation. To check your downloads, run this in
the folder with the files:

```sh
for file in passepartout*; do gh attestation verify "$file" --repo UntoastedToast/passepartout; done
```

## Releasing

Bump `version` in `Cargo.toml` and add a `<release>` entry to the metainfo; CI refuses a tag that
doesn't match. Then push the tag and CI publishes the release:

```sh
git tag vX.Y.Z && git push origin vX.Y.Z
```

## Layout

- `src/engine/` image logic without any GTK: presets, settings, rendering, batch export
- `src/ui/` widgets only: settings panel, image list, preview, main window

## License

Apache-2.0, see `LICENSE`.
