{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        mouse-gestures-package = pkgs.callPackage ./package.nix {};
      in
      {
        packages = rec {
          mouse-gestures = mouse-gestures-package;
          default = mouse-gestures;
        };
        apps = rec {
          mouse-gestures = flake-utils.lib.mkApp {
            drv = self.packages.${system}.unicorn-engine-demo;
          };
          default = mouse-gestures;
        };
        devShells.default = pkgs.mkShell {
          buildInputs =
            mouse-gestures-package.nativeBuildInputs
            ++ (with pkgs; [
              cargo
            ]);
        };
    });
}
