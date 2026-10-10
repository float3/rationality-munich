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

    # No checks: there are no static pages left to walk offline. Every page
    # is written by the program from the live feeds, so CI builds them and
    # runs lychee over the output instead (the HTML job in ci.yml).
    checks = forAllSystems (_: {});

    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [cargo clippy rustc rustfmt];
      };
    });
  };
}
