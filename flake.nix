{
  description = "Nix binary cache garbage collector";

  inputs.nixpkgs.url = github:NixOS/nixpkgs/release-22.05;

  outputs = { self, nixpkgs }: {
    overlay = final: prev: {
      cache-gc =
        let
          nix = final.nixVersions.nix_2_9;
          inherit (final) rustPlatform lib coreutils jq findutils;
        in rustPlatform.buildRustPackage {
          src = lib.cleanSource ./.;
          name = "cache-gc";
          cargoLock.lockFile = ./Cargo.lock;
          buildInputs = [ final.bash ];
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
        };
    };

    defaultPackage.x86_64-linux = self.packages.cache-gc.x86_64-linux;
    packages.cache-gc.x86_64-linux =
      let
        pkgs = import nixpkgs { overlays = [ self.overlay ]; system = "x86_64-linux"; };
      in pkgs.cache-gc;

    devShell.x86_64-linux =
      let
        pkgs = import nixpkgs { system = "x86_64-linux"; };
      in pkgs.mkShell {
        name = "cache-gc-dev";
        buildInputs = with pkgs; [ cargo rustfmt rustc ];
      };

    nixosModules.cache-gc = { config, lib, pkgs, ... }: with lib;
      let
        cfg = config.services.cache-gc;
      in {
        options.services.cache-gc = {
          enable = mkEnableOption "automatic cache gc";
          path = mkOption {
            type = types.str;
            example = "/var/lib/hydra/cache-gc";
            description = ''
              Path containing the binary cache to clean up.
            '';
          };
          days = mkOption {
            default = 90;
            type = types.ints.positive;
            description = ''
              Remove all paths that are older than the <literal>n</literal>
              days and not referenced by paths newer than <literal>n</literal>
              days with <literal>n</literal> being the value of this option.
            '';
          };
          frequency = mkOption {
            default = null;
            type = types.nullOr types.str;
            example = "daily";
            description = ''
              When not <literal>null</literal>, a timer is added which periodically triggers
              the garbage collector. In that case, the option's value must represent
              the frequency of how often the service shuld be executed.

              Further information can be found in <citerefentry><refentrytitle>systemd.time</refentrytitle>
              <manvolnum>7</manvolnum></citerefentry>.
            '';
          };
          owningUser = mkOption {
            type = types.str;
            example = "hydra-queue-runner";
            description = ''
              User which owns the path <option>services.cache-gc.path</option>.
            '';
          };
        };
        config = mkIf cfg.enable {
          nixpkgs.overlays = [ self.overlay ];
          systemd.services.cache-gc = {
            description = "Nix binary cache garbage collector";
            environment.HOME = "/run/cache-gc";
            serviceConfig = {
              CapabilityBoundingSet = [ "" ];
              ExecStart = "${pkgs.cache-gc}/bin/cache-gc --days ${toString cfg.days} --delete ${cfg.path}";
              LockPersonality = true;
              MemoryDenyWriteExecute = true;
              NoNewPrivileges = true;
              PrivateDevices = true;
              PrivateTmp = true;
              ProtectClock = true;
              ProtectControlGroups = true;
              ProtectHome = true;
              ProtectHostname = true;
              ProtectKernelModules = true;
              ProtectKernelLogs = true;
              ProtectKernelTunables = true;
              ProtectSystem = "strict";
              ReadWritePaths = cfg.path;
              RestrictAddressFamilies = "none";
              RestrictNamespaces = true;
              RestrictRealtime = true;
              RestrictSUIDSGID = true;
              RuntimeDirectory = "cache-gc";
              RemoveIPC = true;
              UMask = "0077";
              User = cfg.owningUser;
            };
          };
          systemd.timers.cache-gc = mkIf (cfg.frequency != null) {
            description = "Periodically collect garbage in flat-file cache ${cfg.path}";
            wantedBy = [ "timers.target" ];
            partOf = [ "cache-gc.service" ];
            timerConfig = {
              Persistent = true;
              OnCalendar = cfg.frequency;
            };
          };
        };
      };
  };
}
