{
  rustPlatform,
  pkg-config,
  wayland,
  libxkbcommon,
}:
rustPlatform.buildRustPackage {
  name = "mouse-gestures";
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
    libxkbcommon
    wayland
  ];
}
