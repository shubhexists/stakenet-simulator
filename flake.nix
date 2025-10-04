{
  description = "Stakenet Simulator workspace with Supabase CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rust = pkgs.rust-bin.stable.latest.default;
      in
      {
        packages = {
          cli = pkgs.rustPlatform.buildRustPackage rec {
            pname = "steward-simulator-cli";
            version = "0.1.0";
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "curve25519-dalek-3.2.1" = "sha256-4MF/qaP+EhfYoRETqnwtaCKC1tnUJlBCxeOPCnKrTwQ=";
                "jito-priority-fee-distribution-0.1.6" = "sha256-y1Kr3H7mZc/QDSIf1KRVf0lzPBfPQaClhTBqNEB2dAw=";
                "jito-steward-0.1.0" = "sha256-ukWYBY3eF+USYzV+pRIHQ1jmJFUQcI5vpkSDTRqEe6A=";
              };
            };
            buildAndTestSubdir = "services/cli";

            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.openssl
              pkgs.openssl.dev
            ];

            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };

          epoch-rewards-tracker = pkgs.rustPlatform.buildRustPackage rec {
            pname = "epoch-rewards-tracker";
            version = "0.1.0";
            src = ./.;
            cargoLock = { 
                lockFile = ./Cargo.lock;
                outputHashes = {
                    "curve25519-dalek-3.2.1" = "sha256-4MF/qaP+EhfYoRETqnwtaCKC1tnUJlBCxeOPCnKrTwQ=";
                    "jito-priority-fee-distribution-0.1.6" = "sha256-y1Kr3H7mZc/QDSIf1KRVf0lzPBfPQaClhTBqNEB2dAw=";
                    "jito-steward-0.1.0" = "sha256-ukWYBY3eF+USYzV+pRIHQ1jmJFUQcI5vpkSDTRqEe6A=";
              };
            };
            buildAndTestSubdir = "services/epoch-rewards-tracker";

            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.openssl
              pkgs.openssl.dev
            ];

            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };

          full = pkgs.rustPlatform.buildRustPackage rec {
            pname = "stakenet-simulator";
            version = "0.1.0";
            src = ./.;
            cargoLock = { 
                lockFile = ./Cargo.lock;
                outputHashes = {
                    "curve25519-dalek-3.2.1" = "sha256-4MF/qaP+EhfYoRETqnwtaCKC1tnUJlBCxeOPCnKrTwQ=";
                    "jito-priority-fee-distribution-0.1.6" = "sha256-y1Kr3H7mZc/QDSIf1KRVf0lzPBfPQaClhTBqNEB2dAw=";
                    "jito-steward-0.1.0" = "sha256-ukWYBY3eF+USYzV+pRIHQ1jmJFUQcI5vpkSDTRqEe6A=";
              };
            };

            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.openssl
              pkgs.openssl.dev
            ];

            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };
        };

        devShells = {
          default = pkgs.mkShell {
            buildInputs = [
              rust
              pkgs.supabase-cli
              pkgs.openssl
              pkgs.pkg-config
            ];
            shellHook = ''
              echo "💡 Run 'supabase start' from the root directory"
              export PKG_CONFIG_PATH=${pkgs.openssl.dev}/lib/pkgconfig
            '';
          };
        };
      }
    );
}
