
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem = { pkgs, ... }: {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "steamdex";
          version = "0.1.0";

          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta.mainProgram = "steamdex";
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            rumdl
          ];
        };
      };
    };
}
