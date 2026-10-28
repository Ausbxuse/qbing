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
  writeShellScript,
  mesa,
}:
let
  launcher = writeShellScript "qbing-launch" ''
    # NixOS supplies host drivers here. Other distributions need a matching
    # Nix Mesa closure instead of loading their incompatible system libraries.
    if [ ! -e /etc/NIXOS ] && [ ! -d /run/opengl-driver ]; then
      export __EGL_VENDOR_LIBRARY_FILENAMES=${mesa}/share/glvnd/egl_vendor.d/50_mesa.json''${__EGL_VENDOR_LIBRARY_FILENAMES:+:$__EGL_VENDOR_LIBRARY_FILENAMES}
      export LIBGL_DRIVERS_PATH=${mesa}/lib/dri''${LIBGL_DRIVERS_PATH:+:$LIBGL_DRIVERS_PATH}
      export GBM_BACKENDS_PATH=${mesa}/lib/gbm''${GBM_BACKENDS_PATH:+:$GBM_BACKENDS_PATH}
      export LD_LIBRARY_PATH=${mesa}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
    fi
    exec ${core}/bin/cstimer-native "$@"
  '';
  core = rustPlatform.buildRustPackage {
    pname = "qbing-core";
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
runCommand "qbing-${core.version}"
  {
    nativeBuildInputs = [ makeWrapper ];
    meta = {
      description = "Native smart-cube timer, reconstruction and training with a GNOME companion";
      license = lib.licenses.gpl3Only;
      platforms = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      mainProgram = "qbing";
    };
  }
  ''
      mkdir -p $out/bin $out/share/applications $out/share/icons/hicolor/scalable/apps $out/share/gnome-shell/extensions/cstimer-companion@local $out/lib/systemd/user $out/share/doc/qbing $out/share/licenses/qbing
      ln -s ${launcher} $out/bin/cstimer-native
      ln -s cstimer-native $out/bin/qbing
      makeWrapper $out/bin/cstimer-native $out/bin/qbing-companion --add-flags --companion
      ln -s qbing-companion $out/bin/cstimer-companion
      ln -s qbing $out/bin/facet
      ln -s qbing-companion $out/bin/facet-companion
      cp ${../native/assets/app-icon/qbing.svg} $out/share/icons/hicolor/scalable/apps/qbing.svg
      cp ${../native/gnome}/cstimer-companion@local/* $out/share/gnome-shell/extensions/cstimer-companion@local/
      cp ${../NATIVE.md} $out/share/doc/qbing/NATIVE.md
      cp ${../LICENSE} $out/share/licenses/qbing/GPL-3.0.txt
      cp ${../src/vendor/three/LICENSE} $out/share/licenses/qbing/LICENSE-three.txt
      cp ${../native/shared/src/commonMain/composeResources/files}/LICENSE-* $out/share/licenses/qbing/
      cp ${../native/shared/src/commonMain/composeResources/files}/OFL-* $out/share/licenses/qbing/
      substitute ${../native/systemd/qbing.service.in} $out/lib/systemd/user/facet.service \
        --replace-fail '@LAUNCHER@' "$out/bin/cstimer-native" \
        --replace-fail '@DOCUMENTATION@' "$out/share/doc/qbing/NATIVE.md"
      ln -s facet.service $out/lib/systemd/user/qbing.service
      cat > $out/share/applications/org.cstimer.Native.desktop <<DESKTOP
    [Desktop Entry]
    Type=Application
    Name=Qbing
    Comment=Smart-cube timer and solve analysis
    Exec=$out/bin/qbing
    Icon=$out/share/icons/hicolor/scalable/apps/qbing.svg
    StartupWMClass=org.cstimer.Native
    Terminal=false
    Categories=Education;Sports;
    Actions=Companion;Quit;

    [Desktop Action Companion]
    Name=Companion
    Exec=$out/bin/qbing-companion

    [Desktop Action Quit]
    Name=Quit
    Exec=$out/bin/qbing --quit
    DESKTOP
      cat > $out/share/applications/cstimer-companion.desktop <<DESKTOP
    [Desktop Entry]
    Type=Application
    Name=Qbing Companion
    Comment=Smart-cube companion
    Exec=$out/bin/qbing-companion
    Icon=$out/share/icons/hicolor/scalable/apps/qbing.svg
    Terminal=false
    Categories=Education;Sports;
    DESKTOP
  ''
