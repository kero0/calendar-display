{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    let
      regular-font =
        pkgs:
        if pkgs.stdenv.targetPlatform.system == "aarch64-linux" then
          "${pkgs.noto-fonts}/share/fonts/noto/NotoSans[wdth,wght].ttf"
        else
          "${pkgs.noto-fonts}/share/fonts/noto/NotoSans.ttf";
      # workaround for pkgs.rename being broken on "armv6l-unknown-linux-musleabihf"
      emoji-font =
        pkgs:
        "${
          pkgs.noto-fonts-monochrome-emoji.overrideAttrs (old: {
            installPhase = ''
              runHook preInstall

              install -m444 -Dt $out/share/fonts/noto ofl/notoemoji/*.ttf

              runHook postInstall
            '';
          })
        }/share/fonts/noto/NotoEmoji[wght].ttf";

      supportedSystems = [
        "x86_64-linux"
        "x86_64-darwin"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      mkshell =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
        in
        {
          ${system}.default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              iconv
              pkg-config
              openssl.dev

              # rust specific tooling
              cargo
              clippy
              rust-analyzer
              rustc
              rustfmt
            ];
            RUST_BACKTRACE = "ALL";
            RUST_SRC_PATH = pkgs.rust.packages.stable.rustPlatform.rustLibSrc;

            REGULAR_FONT_PATH = regular-font pkgs;
            EMOJI_FONT_PATH = emoji-font pkgs;
          };

        };
      mkrpi =
        system:
        let
          pkgs = import nixpkgs {
            system = system;
            crossSystem = {
              config = "armv6l-unknown-linux-musleabihf";
            };
          };
          inherit (pkgs) pkgsStatic;
        in
        {
          ${system} = pkgs.rustPlatform.buildRustPackage rec {
            name = "calendar-display";
            version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
            nativeBuildInputs = with pkgsStatic; [
              pkg-config
            ];

            REGULAR_FONT_PATH = regular-font pkgs;
            EMOJI_FONT_PATH = emoji-font pkgs;

            buildInputs = with pkgsStatic; [
              openssl.dev
            ];

            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            RUSTFLAGS = [
              "-C"
              "target-cpu=arm1176jzf-s"
              "-C"
              "target-feature=+crt-static"
            ];
            cargoBuildFlags = [
              "--bin"
              "calendar-display-pi"
              "-F"
              "pi"
            ];
          };
        };
      mkserver =
        system:
        let
          pkgs = import nixpkgs {
            system = system;
          };
        in
        {
          ${system} = pkgs.rustPlatform.buildRustPackage rec {
            name = "calendar-display-server";
            doCheck = false;
            version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            REGULAR_FONT_PATH = regular-font pkgs;
            EMOJI_FONT_PATH = emoji-font pkgs;

            buildInputs = with pkgs; [
              openssl.dev
            ];

            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [
              "--bin"
              "calendar-display-server"
              "--features=server"
            ];
          };
        };
    in
    {
      devShells = nixpkgs.lib.foldr nixpkgs.lib.mergeAttrs { } (map mkshell supportedSystems);
      rpi = nixpkgs.lib.foldr nixpkgs.lib.mergeAttrs { } (map mkrpi supportedSystems);
      server = nixpkgs.lib.foldr nixpkgs.lib.mergeAttrs { } (map mkserver supportedSystems);
    };
}
