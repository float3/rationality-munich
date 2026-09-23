{
  description = "rationality-munich.com: the pages and the calendar generator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    lib = nixpkgs.lib;
    systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forAllSystems = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
  in {
    formatter = forAllSystems (pkgs: pkgs.alejandra);

    packages = forAllSystems (pkgs: {
      default = pkgs.rustPlatform.buildRustPackage {
        pname = "rationality-calendar";
        version = "0.2.0";
        src = ./calendar;
        cargoLock.lockFile = ./calendar/Cargo.lock;
        meta.mainProgram = "rationality-calendar";
      };
    });

    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [cargo clippy rustc rustfmt];
      };
    });
  };
}
