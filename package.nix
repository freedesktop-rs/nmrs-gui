{
  lib,
  stdenv,
  rustPlatform,
  glib-networking,
  pkg-config,
  wrapGAppsHook4,
  libxkbcommon,
  wayland,
  glib,
  gobject-introspection,
  gtk4,
  libadwaita,
}:

rustPlatform.buildRustPackage {
  pname = "nmrs-gui";
  version = "1.5.1";

  src = ./.;

  cargoHash = "sha256-/i9+33zs6wJWMZfjYhRg/SzYD0n7XuuD0FbbnOMyfQg=";

  nativeBuildInputs = [
    pkg-config
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [ wrapGAppsHook4 ];

  buildInputs = lib.optionals stdenv.hostPlatform.isLinux [
    glib-networking
    libxkbcommon
    wayland
    glib
    gobject-introspection
    gtk4
    libadwaita
  ];

  doCheck = false;
  doInstallCheck = true;

  postInstall = ''
    install -D nmrs.desktop -t $out/share/applications
  '';

  meta = with lib; {
    description = "GTK4 GUI for managing NetworkManager connections";
    homepage = "https://github.com/networkmanager-rs/nmrs-gui";
    license = licenses.mit;
    maintainers = [ ];
    mainProgram = "nmrs-gui";
    platforms = platforms.linux;
  };
}