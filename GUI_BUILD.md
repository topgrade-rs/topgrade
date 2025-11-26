# Building Topgrade GUI

## System Dependencies

**No system dependencies required!** The GUI uses `egui`, which is a pure Rust immediate mode GUI library that doesn't require any system libraries to be installed.

## Building

Build the GUI binary with:
```bash
cargo build --bin topgrade-gui --features gui --release
```

Or install it:
```bash
cargo install --path . --bin topgrade-gui --features gui
```

The binary will be available at `target/release/topgrade-gui` (or `target/debug/topgrade-gui` for debug builds).

## Desktop File Installation

After building, you can install the desktop file to make it available in your application menu:

```bash
# For user installation
mkdir -p ~/.local/share/applications
cp topgrade-gui.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications/

# For system-wide installation (requires root)
sudo cp topgrade-gui.desktop /usr/share/applications/
sudo update-desktop-database
```

Make sure the icon file `doc/topgrade.png` is accessible. You may need to install it to a system icon directory:

```bash
# For user installation
mkdir -p ~/.local/share/icons/hicolor/256x256/apps
cp doc/topgrade.png ~/.local/share/icons/hicolor/256x256/apps/topgrade.png

# For system-wide installation
sudo mkdir -p /usr/share/icons/hicolor/256x256/apps
sudo cp doc/topgrade.png /usr/share/icons/hicolor/256x256/apps/topgrade.png
sudo gtk-update-icon-cache /usr/share/icons/hicolor/
```

## Running

You can run the GUI either:
- From the command line: `topgrade-gui`
- From your system's application menu (after installing the desktop file)
