{ sources ? import ./nix/sources.nix
, nixpkgs ? sources.nixpkgs.outPath
, pkgs ? import (nixpkgs + "/pkgs/top-level/default.nix") {localSystem = builtins.currentSystem;}
, lib ? pkgs.lib
, fetchpatch ? pkgs.fetchpatch
, rustPlatform ? pkgs.rustPlatform
, jq ? pkgs.jq
, nix ? pkgs.nix_2_4
, runtimeShell ? pkgs.runtimeShell
, coreutils ? pkgs.coreutils
, findutils ? pkgs.findutils
}:
let
  nix_with_patch = nix.overrideAttrs (o: {
    patches = [(fetchpatch {
      name = "nix-path-info-fix.patch";
      url = "https://github.com/NixOS/nix/commit/dbdc63bc41d33a33e75d5fc8efa8e6520f9e6494.patch";
      sha256 = "16gas3axw3inn1fnsxqgnnk38jfcxisnzq57jb6y435iy6j4sy46";
    })];
  });
in rustPlatform.buildRustPackage {
  src = lib.cleanSource ./.;
  name = "cache-gc";
  cargoLock.lockFile = ./Cargo.lock;
  postInstall = ''
    mkdir -p $out/libexec/cache-gc
    mv $out/bin/gc $out/libexec/cache-gc/
    cp $src/add-registration-times.jq $out/libexec/cache-gc
    install -m0755 $src/run.sh $out/bin/cache-gc
    sed -i $out/bin/cache-gc \
      -e "2iexport PATH=$out/libexec/cache-gc:${lib.makeBinPath [ coreutils jq nix_with_patch findutils ]}" \
      -e "2ilibexec_dir=$out/libexec/cache-gc"
    patchShebangs $out/bin/cache-gc
  '';
}
