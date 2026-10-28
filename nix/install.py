"""Install the Nix closure and per-user desktop integration, preserving all cube data."""
import argparse
import os
from pathlib import Path
import subprocess


def link(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + '.facet-new')
    temporary.unlink(missing_ok=True)
    temporary.symlink_to(source)
    os.replace(temporary, destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    parser.add_argument('--root', type=Path, help='Install under a separate root (for isolated installation tests)')
    args = parser.parse_args()
    home = args.root.resolve() if args.root else Path.home()
    def xdg(variable, suffix):
        return home / suffix if args.root else Path(os.environ.get(variable, home / suffix))
    data = xdg('XDG_DATA_HOME', '.local/share')
    state = xdg('XDG_STATE_HOME', '.local/state')
    config = xdg('XDG_CONFIG_HOME', '.config')
    profile = state / 'facet/profile'
    profile.parent.mkdir(parents=True, exist_ok=True)
    # A dedicated, atomic Nix profile retains the complete closure and rollback
    # generations without changing the user's other installed packages.
    subprocess.run(['nix-env', '--profile', str(profile), '--set', str(args.package)], check=True)
    for name in ['facet', 'facet-companion', 'cstimer-native', 'cstimer-companion']:
        link(profile / 'bin' / name, home / '.local/bin' / name)
    link(profile / 'share/applications/org.cstimer.Native.desktop', data / 'applications/org.cstimer.Native.desktop')
    extension_id = 'cstimer-companion@local'
    extension = data / 'gnome-shell/extensions' / extension_id
    extension.mkdir(parents=True, exist_ok=True)
    for name in ['extension.js', 'notation.js', 'stylesheet.css', 'metadata.json']:
        link(profile / 'share/gnome-shell/extensions' / extension_id / name, extension / name)
    units = config / 'systemd/user'
    link(profile / 'lib/systemd/user/facet.service', units / 'facet.service')
    link('../facet.service', units / 'graphical-session.target.wants/facet.service')
    if not args.root:
        subprocess.run(['systemctl', '--user', 'daemon-reload'], check=False)
        enabled = subprocess.run(['gsettings', 'get', 'org.gnome.shell', 'enabled-extensions'], capture_output=True, text=True)
        if enabled.returncode == 0 and "'" + extension_id + "'" not in enabled.stdout:
            value = enabled.stdout.strip().removeprefix('@as ')
            value = "['" + extension_id + "']" if value == '[]' else value[:-1] + ", '" + extension_id + "']"
            subprocess.run(['gsettings', 'set', 'org.gnome.shell', 'enabled-extensions', value], check=True)
    print('Facet installed. Run facet, or open Facet from Applications.')
    print('The GNOME companion loads at your next graphical login. Existing cube data and running connections were preserved.')


if __name__ == '__main__':
    main()
