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
          glib-networking
          gobject-introspection
          gtk4
          libadwaita
          libxkbcommon
          wayland
          glib
        ];

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

            shellHook = ''
              alias ls=eza
              alias find=fd
            '';
          };

        devShell = self.devShells.${system}.default;

        packages.default = naersk-package.buildPackage {
          pname = "nmrs-gui";
          version = cargoToml.package.version;
          src = ./.;

          nativeBuildInputs =
            nativeBuildInputs
            ++ (with pkgs; [
              wrapGAppsHook4
            ]);
          inherit buildInputs;

          postInstall = ''
            install -D nmrs.desktop -t $out/share/applications
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
