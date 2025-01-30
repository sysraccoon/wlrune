{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
      };
      lib = pkgs.lib;
      wlrune-package = pkgs.callPackage ./package.nix {};
    in {
      packages = rec {
        wlrune = wlrune-package;
        default = wlrune;
      };

      apps = rec {
        wlrune = flake-utils.lib.mkApp {
          drv = self.packages.${system}.wlrune;
        };
        default = wlrune;
      };

      devShells.default = pkgs.mkShell rec {
        buildInputs =
          wlrune-package.nativeBuildInputs
          ++ wlrune-package.buildInputs
          ++ (with pkgs; [
            rustup
            cargo
            cargo-bloat

            libxkbcommon

            wayland
            wayland-scanner
            wayland-protocols
          ]);

        LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        RUST_BACKTRACE = "full";
      };
    });
}
