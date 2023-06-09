{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages."${system}";
        rust = fenix.packages.${system}.stable;
        craneLib = crane.lib."${system}".overrideToolchain rust.toolchain;
        buildInputs = with pkgs; [
          alsaLib
          udev
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          libxkbcommon
          vulkan-loader
          wayland
        ];
        nativeBuildInputs = with pkgs; [
          mold
          clang
          pkg-config
        ];
        rustsrc = craneLib.cleanCargoSource (craneLib.path ./.);
      in {
        packages.basebuilder-bin = craneLib.buildPackage {
          name = "basebuilder-bin";
          src = rustsrc;
          inherit buildInputs;
          inherit nativeBuildInputs;
        };

        packages.basebuilder-assets = pkgs.stdenv.mkDerivation {
          name = "basebuilder-assets";
          src = ./assets;
          phases = ["unpackPhase" "installPhase"];
          installPhase = ''
            mkdir -p $out
            cp -r $src $out/assets
          '';
        };

        packages.basebuilder = pkgs.stdenv.mkDerivation {
          name = "basebuilder";
          phases = ["installPhase"];
          installPhase = ''
            mkdir -p $out
            ln -s ${self.packages.${system}.basebuilder-assets}/assets $out/assets
            cp ${self.packages.${system}.basebuilder-bin}/bin/basebuilder $out/basebuilder
          '';
        };

        packages.basebuilder-wasm = let
          target = "wasm32-unknown-unknown";
          toolchainWasm = with fenix.packages.${system};
            combine [
              stable.rustc
              stable.cargo
              targets.${target}.stable.rust-std
            ];
          craneWasm = crane.lib.${system}.overrideToolchain toolchainWasm;
        in
          craneWasm.buildPackage {
            src = rustsrc;
            CARGO_BUILD_TARGET = target;
            CARGO_PROFILE = "release";
            inherit nativeBuildInputs;
            doCheck = false;
          };

        packages.basebuilder-web = pkgs.stdenv.mkDerivation {
          name = "basebuilder-web";
          src = ./web;
          nativeBuildInputs = [
            pkgs.wasm-bindgen-cli
            pkgs.binaryen
          ];
          phases = ["unpackPhase" "installPhase"];
          installPhase = ''
            mkdir -p $out
            wasm-bindgen --out-dir $out --out-name basebuilder --target web ${self.packages.${system}.basebuilder-wasm}/bin/basebuilder.wasm
            mv $out/basebuilder_bg.wasm .
            wasm-opt -Oz -o $out/basebuilder_bg.wasm basebuilder_bg.wasm
            cp * $out/
            ln -s ${self.packages.${system}.basebuilder-assets}/assets $out/assets
          '';
        };

        packages.basebuilder-web-server = pkgs.writeShellScriptBin "basebuilder-web-server" ''
          ${pkgs.simple-http-server}/bin/simple-http-server -i -c=html,wasm,ttf,js -- ${self.packages.${system}.basebuilder-web}/
        '';

        defaultPackage = self.packages.${system}.basebuilder;

        apps.basebuilder = flake-utils.lib.mkApp {
          drv = self.packages.${system}.basebuilder;
          exePath = "/basebuilder";
        };

        apps.basebuilder-web-server = flake-utils.lib.mkApp {
          drv = self.packages.${system}.basebuilder-web-server;
          exePath = "/bin/basebuilder-web-server";
        };

        defaultApp = self.apps.${system}.basebuilder;

        checks = {
          pre-commit-check = inputs.pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              alejandra.enable = true;
              statix.enable = true;
              rustfmt.enable = true;
              clippy = {
                enable = false;
                entry = let
                  rust-clippy = rust-clippy.withComponents ["clippy"];
                in
                  pkgs.lib.mkForce "${rust-clippy}/bin/cargo-clippy clippy";
              };
            };
          };
        };

        devShell = pkgs.mkShell {
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath buildInputs}"
            ${self.checks.${system}.pre-commit-check.shellHook}
          '';
          inherit buildInputs;
          nativeBuildInputs =
            [
              (rust.withComponents ["cargo" "rustc" "rust-src" "rustfmt" "clippy"])
            ]
            ++ nativeBuildInputs;
        };
      }
    );
}
