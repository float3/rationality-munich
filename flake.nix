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

    # Links between the static pages, and to their anchors. Offline: builds
    # have no network, so links to other sites are CI's job (the Links job
    # in .github/workflows/ci.yml). /calendar/ is generated on the server;
    # CI checks those links against pages it generates.
    checks = forAllSystems (pkgs: {
      links =
        pkgs.runCommand "rationality-munich-links" {
          nativeBuildInputs = [pkgs.lychee];
        } ''
          export HOME=$TMPDIR SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
          cd ${./www}
          lychee --offline --no-progress --include-fragments \
            --root-dir "$PWD" \
            --remap "https://rationality-munich\.com/(.*) file://$PWD/\$1" \
            --exclude '/calendar' \
            *.html sitemap.xml robots.txt .well-known/security.txt
          touch $out
        '';
    });

    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [cargo clippy rustc rustfmt];
      };
    });
  };
}
