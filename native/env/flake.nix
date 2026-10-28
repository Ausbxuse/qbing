{
  description = "Cube Lab native Android and desktop development";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }: let
    systems = [ "x86_64-linux" "aarch64-linux" ];
    eachSystem = nixpkgs.lib.genAttrs systems;
  in {
    devShells = eachSystem (system: let
      pkgs = import nixpkgs { inherit system; config = { allowUnfree = true; android_sdk.accept_license = true; }; };
      android = pkgs.androidenv.composeAndroidPackages {
        platformVersions = [ "36" ]; buildToolsVersions = [ "36.0.0" ];
        includeEmulator = false; includeNDK = false; includeSystemImages = false;
      };
    in { default = pkgs.mkShell {
      packages = [ (pkgs.python3.withPackages (ps: [ ps.fonttools ps.brotli ])) pkgs.jdk21 android.androidsdk pkgs.nodejs_24 pkgs.dbus pkgs.pkg-config pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.wayland pkgs.libxkbcommon pkgs.xorg.libXcursor pkgs.mesa pkgs.xorg.xorgserver ];
      JAVA_HOME = "${pkgs.jdk21}";
      ANDROID_HOME = "${android.androidsdk}/libexec/android-sdk";
      ANDROID_SDK_ROOT = "${android.androidsdk}/libexec/android-sdk";
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.libpulseaudio pkgs.alsa-lib pkgs.nss pkgs.nspr pkgs.libxcb pkgs.expat pkgs.libdrm pkgs.libGL pkgs.wayland pkgs.libxkbcommon pkgs.xorg.libXcursor pkgs.xorg.libXrandr pkgs.xorg.libX11 pkgs.xorg.libXext pkgs.xorg.libXi pkgs.xorg.libXrender pkgs.xorg.libXtst pkgs.fontconfig pkgs.freetype ];
      shellHook = ''
        export GRADLE_OPTS="-Dorg.gradle.project.android.aapt2FromMavenOverride=$ANDROID_HOME/build-tools/36.0.0/aapt2"
      '';
    }; });
  };
}
