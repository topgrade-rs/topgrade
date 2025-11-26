# Building Topgrade GUI

## System Dependencies

To build the GUI version of topgrade, you need to install the following system dependencies:

### Ubuntu/Debian
```bash
sudo apt install libgtk-4-dev libgdk-pixbuf-2.0-dev libglib2.0-dev pkg-config
```

### Fedora/RHEL/CentOS
```bash
sudo dnf install gtk4-devel gdk-pixbuf2-devel glib2-devel pkg-config
```

### Arch Linux
```bash
sudo pacman -S gtk4 gdk-pixbuf2 glib2 pkg-config
```

## Building

Build the GUI binary with:
```bash
cargo build --bin topgrade-gui --features gui --release
```

Or install it:
```bash
cargo install --path . --bin topgrade-gui --features gui
```

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

