{
  description = "nmrs-gui development flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    naersk.url = "github:nix-community/naersk/master";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    naersk,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };
        inherit (pkgs) lib;
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        cargoToml = lib.importTOML ./Cargo.toml;

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];
        buildInputs = with pkgs; [
          fontconfig
          freetype
          libxkbcommon
          wayland
          vulkan-loader
          libX11
          libxcb
        ];

        runtimeLibs = lib.makeLibraryPath buildInputs;

        naersk-package = pkgs.callPackage naersk {
          cargo = toolchain;
          rustc = toolchain;
          clippy = toolchain;
        };
      in {
        devShells.default = with pkgs;
          mkShell {
            packages =
              [
                toolchain
                cargo-info
                rust-analyzer
                just

                eza
                fd
                fzf
                bat
              ]
              ++ nativeBuildInputs
              ++ buildInputs;
            RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
            LD_LIBRARY_PATH = runtimeLibs;

            shellHook = ''
              alias ls=eza
              alias find=fd
            '';
          };

        devShell = self.devShells.${system}.default;
        checks.build = self.packages.${system}.default;
        packages.default = naersk-package.buildPackage {
          pname = "nmrs-gui";
          version = cargoToml.package.version;
          src = ./.;

          nativeBuildInputs = nativeBuildInputs ++ [pkgs.makeWrapper];
          inherit buildInputs;

          postInstall = ''
            install -D nmrs.desktop -t $out/share/applications
            install -Dm644 assets/nmrs.svg -t $out/share/icons/hicolor/scalable/apps
            wrapProgram $out/bin/nmrs-gui --prefix LD_LIBRARY_PATH : ${runtimeLibs}
          '';

          meta = with lib; {
            description = cargoToml.package.description;
            homepage = cargoToml.package.repository;
            license = licenses.mit;
            mainProgram = "nmrs-gui";
            platforms = platforms.linux;
          };
        };
      }
    );
}
