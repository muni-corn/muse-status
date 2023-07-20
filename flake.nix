{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, naersk, rust-overlay }:
    let allSystems =
      flake-utils.lib.eachDefaultSystem (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };

          rust = pkgs.rust-bin.nightly.latest.default;

          naersk-lib = naersk.lib."${system}".override {
            cargo = rust;
            rustc = rust;
          };

          nativeBuildInputs = builtins.attrValues {
            inherit rust;

            inherit (pkgs)
              dbus
              pkg-config
              rustfmt
              libressl_3_6;
          };
          buildInputs = with pkgs; [ dbus pamixer alsa-utils iputils ];

          muse-status = naersk-lib.buildPackage {
            pname = "muse-status";
            root = builtins.path {
              path = ./.;
              name = "muse-status-src";
            };
            inherit nativeBuildInputs buildInputs;
          };

          muse-status-client-app = flake-utils.lib.mkApp {
            name = "muse-status-client";
            drv = muse-status;
            exePath = "/bin/muse-status";
          };

          muse-status-daemon-app = flake-utils.lib.mkApp {
            name = "muse-status-daemon";
            drv = muse-status;
            exePath = "/bin/muse-status-daemon";
          };
        in
        {
          # `nix build`
          packages = { inherit muse-status; };
          defaultPackage = muse-status;

          # `nix run`
          apps.muse-status = muse-status-client-app;
          apps.muse-status-daemon = muse-status-daemon-app;

          # `nix develop`
          devShell =
            let
              inherit (pkgs) mkShell cargo-watch clippy rust-analyzer rustfmt;
            in
            mkShell {
              nativeBuildInputs = nativeBuildInputs ++ buildInputs ++ [
                cargo-watch
                clippy
                rust-analyzer
                rustfmt
              ];
            };
        }
      );
    in
    {
      inherit (allSystems) packages defaultPackage apps devShell;
      overlay = final: prev: {
        muse-status = allSystems.packages.${final.system}.muse-status;
      };
    };
}
