{
  rustPlatform,
}:

rustPlatform.buildRustPackage rec {
  name = "mouse-gestures";
  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  nativeBuildInputs = [
  ];
}

