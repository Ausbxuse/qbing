{
  lib,
  rustPlatform,
  pkg-config,
  makeWrapper,
  dbus,
  libGL,
  wayland,
  libxkbcommon,
  libxcursor,
  libxrandr,
  libx11,
  libxi,
  libxext,
  libxrender,
  libxfixes,
  fontconfig,
  freetype,
  runCommand,
}:
let
  core = rustPlatform.buildRustPackage {
    pname = "facet-core";
    version = "0.1.0";
    src = lib.fileset.toSource {
      root = ../.;
      fileset = lib.fileset.unions [
        ../native/linux/Cargo.toml
        ../native/linux/Cargo.lock
        ../native/linux/src
        ../native/linux/assets
        ../native/assets
        ../native/shared/src/commonMain/composeResources/files
        ../native/androidApp/src/debug/assets/analysis-fixture.json
      ];
    };
    cargoRoot = "native/linux";
    buildAndTestSubdir = "native/linux";
    cargoHash = "sha256-MHg09R8K+/scq7t7j07sxoxytc8x8LVx7Zf3piypA8g=";
    nativeBuildInputs = [
      pkg-config
      makeWrapper
    ];
    buildInputs = [ dbus ];
    cargoTestFlags = [
      "--bin"
      "cstimer-native"
    ];
    postInstall = ''
      wrapProgram $out/bin/cstimer-native --prefix LD_LIBRARY_PATH : ${
        lib.makeLibraryPath [
          libGL
          wayland
          libxkbcommon
          libxcursor
          libxrandr
          libx11
          libxi
          libxext
          libxrender
          libxfixes
          fontconfig
          freetype
        ]
      }
    '';
  };
in
runCommand "facet-${core.version}"
  {
    nativeBuildInputs = [ makeWrapper ];
    meta = {
      description = "Native smart-cube timer, reconstruction and training with a GNOME companion";
      license = lib.licenses.gpl3Only;
      platforms = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      mainProgram = "facet";
    };
  }
  ''
      mkdir -p $out/bin $out/share/applications $out/share/icons/hicolor/scalable/apps $out/share/gnome-shell/extensions/cstimer-companion@local $out/lib/systemd/user $out/share/doc/facet $out/share/licenses/facet
      ln -s ${core}/bin/cstimer-native $out/bin/cstimer-native
      ln -s cstimer-native $out/bin/facet
      makeWrapper $out/bin/cstimer-native $out/bin/facet-companion --add-flags --companion
      ln -s facet-companion $out/bin/cstimer-companion
      cp ${../native/assets/app-icon/facet.svg} $out/share/icons/hicolor/scalable/apps/facet.svg
      cp ${../native/gnome}/cstimer-companion@local/* $out/share/gnome-shell/extensions/cstimer-companion@local/
      cp ${../NATIVE.md} $out/share/doc/facet/NATIVE.md
      cp ${../LICENSE} $out/share/licenses/facet/GPL-3.0.txt
      cp ${../src/vendor/three/LICENSE} $out/share/licenses/facet/LICENSE-three.txt
      cp ${../native/shared/src/commonMain/composeResources/files}/LICENSE-* $out/share/licenses/facet/
      cp ${../native/shared/src/commonMain/composeResources/files}/OFL-* $out/share/licenses/facet/
      substitute ${../native/systemd/facet.service.in} $out/lib/systemd/user/facet.service \
        --replace-fail '@LAUNCHER@' "$out/bin/cstimer-native" \
        --replace-fail '@DOCUMENTATION@' "$out/share/doc/facet/NATIVE.md"
      cat > $out/share/applications/org.cstimer.Native.desktop <<DESKTOP
    [Desktop Entry]
    Type=Application
    Name=Facet
    Comment=Smart-cube timer and solve analysis
    Exec=$out/bin/facet
    Icon=$out/share/icons/hicolor/scalable/apps/facet.svg
    StartupWMClass=org.cstimer.Native
    Terminal=false
    Categories=Education;Sports;
    Actions=Companion;Quit;

    [Desktop Action Companion]
    Name=Companion
    Exec=$out/bin/facet-companion

    [Desktop Action Quit]
    Name=Quit
    Exec=$out/bin/facet --quit
    DESKTOP
  ''
