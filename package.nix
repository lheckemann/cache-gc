{ nix, rustPlatform, lib, coreutils, jq, findutils, bash, src ? lib.cleanSource ./. }:
rustPlatform.buildRustPackage {
  inherit src;
  pname = "cache-gc";
  version = "unstable" + lib.optionalString ((src.sourceInfo or {}) ? lastModifiedDate) "-${lib.substring 0 8 src.sourceInfo.lastModifiedDate}";
  cargoLock.lockFile = ./Cargo.lock;
  buildInputs = [ bash ];
  postInstall = ''
    mkdir -p $out/libexec/cache-gc
    mv $out/bin/gc $out/libexec/cache-gc/
    cp $src/add-registration-times.jq $out/libexec/cache-gc
    install -m0755 $src/run.sh $out/bin/cache-gc
    sed -i $out/bin/cache-gc \
      -e "2iexport PATH=$out/libexec/cache-gc:${lib.makeBinPath [ coreutils jq nix findutils ]}" \
      -e "2ilibexec_dir=$out/libexec/cache-gc"
    patchShebangs $out/bin/cache-gc
  '';
}
