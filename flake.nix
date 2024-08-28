{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
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
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {inherit system overlays;};

      inherit (pkgs) stdenv;

      rustPlatform = pkgs.makeRustPlatform {
        cargo = pkgs.rust-bin.stable.latest.minimal;
        rustc = pkgs.rust-bin.stable.latest.minimal;
      };

      cargo-tauri2 = rustPlatform.buildRustPackage rec {
        pname = "tauri";
        version = "2.0.0-rc.6";

        src = pkgs.fetchFromGitHub {
          owner = "tauri-apps";
          repo = "tauri";
          rev = "07aff5a2d422877b498bf8865ebdf15d36f1fd18";
          hash = "sha256-y8Y9En1r1HU9sZcYHFhB+botVQBZfzqoDrlgp98ltrY=";
        };

        sourceRoot = "${src.name}/tooling/cli";
        cargoHash = "sha256-lNAWyd7EtjzTFsadBKuD7Y73UUNOU7OS6E1X8mPRUb4=";

        buildInputs = with pkgs; [openssl glibc libsoup cairo gtk3 webkitgtk_4_1];
        nativeBuildInputs = with pkgs; [pkg-config];
      };

      libraries = with pkgs; [
        gtk3
        gtk-layer-shell
        cairo
        gdk-pixbuf
        glib
        dbus
        librsvg
      ];

      packages = with pkgs; [
        webkitgtk_4_1
        pkg-config
        dbus
        glib
        gtk3
        gtk-layer-shell
        libsoup
        librsvg
        pam
        libxkbcommon
      ];
    in {
      packages.default = stdenv.mkDerivation (finalAttrs: {
        pname = "dash2";
        version = "0.1.0";
        src = ./.;

        nativeBuildInputs = with pkgs;
          [
            cargo
            rustc
            cargo-tauri2
            nodejs
            pnpm.configHook
            copyDesktopItems
            pkg-config
            libxkbcommon
            pam
          ]
          ++ [
            rustPlatform.cargoSetupHook
          ];

        buildInputs = with pkgs; [
          openssl
          webkitgtk_4_1
          gtk-layer-shell
        ];

        pnpmDeps = pkgs.pnpm.fetchDeps {
          inherit (finalAttrs) pname version src;
          hash = "sha256-OP9mPHCOiXSKhahbQ2DTNAUhLD9VptnLE4oIha/aPvU=";
        };

        cargoRoot = "src-tauri/";
        cargoDeps = rustPlatform.importCargoLock {
          lockFile = ./src-tauri/Cargo.lock;
        };

        preBuild = ''
          cargo tauri build
        '';

        preInstall = ''
          echo "hello i'm preinstalling"
          echo "out is $out"

          ${pkgs.tree}/bin/tree .
          mv src-tauri/target/release/bundle/deb/*/data/usr/ $out
          rm -r $out/share/applications
        '';

        # installPhase = ''
        #   echo "hello i'm installing"
        #   echo "out is $out"
        #
        #
        #   runHook postInstall
        # '';
      });

      devShell = pkgs.mkShell {
        buildInputs = packages;

        shellHook = ''
          export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
          export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS
        '';
      };
    });
}
