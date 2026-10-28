{
  description = "Qbing — native smart-cube timer, analysis and GNOME companion";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs =
    { self, nixpkgs }:
    let
      linuxSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      eachLinux = nixpkgs.lib.genAttrs linuxSystems;
    in
    {
      packages = eachLinux (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          qbing = pkgs.callPackage ./nix/package.nix { };
          installer = pkgs.writeShellApplication {
            name = "install-qbing";
            runtimeInputs = [
              pkgs.nix
              pkgs.python3
              pkgs.systemd
              pkgs.glib
            ];
            text = ''exec python3 ${./nix/install.py} ${qbing} "$@"'';
          };
        in
        {
          inherit qbing installer;
          default = qbing;
        }
      );
      apps = eachLinux (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.qbing}/bin/qbing";
          meta.description = "Open Qbing";
        };
        install = {
          type = "app";
          program = "${self.packages.${system}.installer}/bin/install-qbing";
          meta.description = "Install Qbing and the GNOME companion for this user";
        };
      });
      checks = eachLinux (system: {
        qbing = self.packages.${system}.qbing;
      });
      devShells = eachLinux (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
            packages = [
              pkgs.jdk21
              pkgs.php
              pkgs.nodejs_24
              pkgs.gnumake
              pkgs.cargo
              pkgs.rustc
              pkgs.rustfmt
              pkgs.pkg-config
              pkgs.wayland
              pkgs.libxkbcommon
              pkgs.mesa
              pkgs.libxcursor
              pkgs.libxrandr
              pkgs.libxi
              pkgs.xorg-server
            ];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
              pkgs.libGL
              pkgs.wayland
              pkgs.libxkbcommon
              pkgs.libxcursor
              pkgs.libxrandr
              pkgs.libx11
              pkgs.libxext
              pkgs.libxi
            ];
          };
        }
      );
    };
}
