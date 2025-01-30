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
      wlrune-static-package = pkgs.pkgsStatic.callPackage ./package.nix {};
    in {
      packages = rec {
        wlrune = wlrune-package;
        wlrune-static = wlrune-static-package;
        default = wlrune;
      };

      apps = rec {
        wlrune = flake-utils.lib.mkApp {
          drv = self.packages.${system}.wlrune;
        };
        wlrune-static = flake-utils.lib.mkApp {
          drv = self.packages.${system}.wlrune-static;
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
          ]);

        LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        RUST_BACKTRACE = "full";
      };
    });
}
