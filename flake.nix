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
      mouse-gestures-package = pkgs.callPackage ./package.nix {};
    in {
      packages = rec {
        mouse-gestures = mouse-gestures-package;
        default = mouse-gestures;
      };

      apps = rec {
        mouse-gestures = flake-utils.lib.mkApp {
          drv = self.packages.${system}.mouse-gestures;
        };
        default = mouse-gestures;
      };

      devShells.default = pkgs.mkShell rec {
        buildInputs =
          mouse-gestures-package.nativeBuildInputs
          ++ mouse-gestures-package.buildInputs
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
