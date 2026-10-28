{
  description = "Facet — native smart-cube timer, analysis and GNOME companion";
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
          facet = pkgs.callPackage ./nix/package.nix { };
          installer = pkgs.writeShellApplication {
            name = "install-facet";
            runtimeInputs = [
              pkgs.nix
              pkgs.python3
              pkgs.systemd
              pkgs.glib
            ];
            text = ''exec python3 ${./nix/install.py} ${facet} "$@"'';
          };
        in
        {
          inherit facet installer;
          default = facet;
        }
      );
      apps = eachLinux (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.facet}/bin/facet";
          meta.description = "Open Facet";
        };
        install = {
          type = "app";
          program = "${self.packages.${system}.installer}/bin/install-facet";
          meta.description = "Install Facet and the GNOME companion for this user";
        };
      });
      checks = eachLinux (system: {
        facet = self.packages.${system}.facet;
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
