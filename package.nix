{ nix, rustPlatform, lib, coreutils, findutils, bash, src ? lib.cleanSource ./., python3 }:
rustPlatform.buildRustPackage {
  inherit src;
  pname = "cache-gc";
  version = "unstable" + lib.optionalString ((src.sourceInfo or {}) ? lastModifiedDate) "-${lib.substring 0 8 src.sourceInfo.lastModifiedDate}";
  cargoLock.lockFile = ./Cargo.lock;
  buildInputs = [ bash python3 ];
  postInstall = ''
    mkdir -p $out/libexec/cache-gc
    mv $out/bin/gc $out/libexec/cache-gc/
    cp $src/add-registration-times.py $out/libexec/cache-gc
    patchShebangs $out/libexec/cache-gc
    install -m0755 $src/run.sh $out/bin/cache-gc
    sed -i $out/bin/cache-gc \
      -e "2iexport PATH=$out/libexec/cache-gc:${lib.makeBinPath [ coreutils nix findutils ]}" \
      -e "2ilibexec_dir=$out/libexec/cache-gc"
    patchShebangs $out/bin/cache-gc
  '';
}
