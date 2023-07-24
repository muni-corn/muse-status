{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    fenix.url = "github:nix-community/fenix";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, naersk }:
    let allSystems =
      flake-utils.lib.eachDefaultSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };

          rust = fenix.packages.${system}.complete;

          naersk-lib = naersk.lib."${system}".override {
            inherit (rust) cargo rustc;
          };

          nativeBuildInputs = [
            rust.toolchain
          ] ++ (with pkgs; [
            dbus
            pkg-config
            rustfmt
            libressl_3_6
          ]);
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
          packages.default = muse-status;

          # `nix run`
          apps = {
            muse-status = muse-status-client-app;
            muse-status-daemon = muse-status-daemon-app;
          };

          # `nix develop`
          devShell =
            pkgs.mkShell {
              inherit nativeBuildInputs buildInputs;
            };
        }
      );
    in
    {
      inherit (allSystems) packages apps devShell;
      overlay = final: prev: {
        muse-status = allSystems.packages.${prev.system}.default;
      };
    };
}
