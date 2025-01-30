{
  rustPlatform,
  pkg-config,
  wayland,
  libxkbcommon,
}:
rustPlatform.buildRustPackage {
  name = "wlrune";
  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  buildInputs = [
    libxkbcommon
    wayland
  ];

  nativeBuildInputs = [
    pkg-config
  ];
}
