{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  makeWrapper,
  fontconfig,
  freetype,
  libxkbcommon,
  wayland,
  vulkan-loader,
  xorg,
}:

let
  runtimeLibs = [
    fontconfig
    freetype
    libxkbcommon
    wayland
    vulkan-loader
    xorg.libX11
    xorg.libxcb
  ];
in

rustPlatform.buildRustPackage {
  pname = "nmrs-gui";
  version = "1.5.1";

  src = ./.;

  cargoHash = "sha256-IsEAdWHwVC35RaEuku42JDP/koiHCcf24HO7WhH3V9A=";

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];

  buildInputs = lib.optionals stdenv.hostPlatform.isLinux runtimeLibs;

  doCheck = false;
  doInstallCheck = true;

  postInstall = ''
    install -D nmrs.desktop -t $out/share/applications
    wrapProgram $out/bin/nmrs-gui --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
  '';

  meta = with lib; {
    description = "A NetworkManager client built on nmrs and gpui";
    homepage = "https://github.com/networkmanager-rs/nmrs-gui";
    license = licenses.mit;
    maintainers = [ ];
    mainProgram = "nmrs-gui";
    platforms = platforms.linux;
  };
}