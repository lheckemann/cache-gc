{
  description = "Nix binary cache garbage collector";

  inputs.nixpkgs.url = github:NixOS/nixpkgs/nixos-23.11;

  outputs = { self, nixpkgs }: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-linux" "aarch64-linux"];
  in {
    overlays.default = final: prev: {
      cache-gc = prev.callPackage ./package.nix { nix = final.nixVersions.nix_2_19; src = self; };
    };

    defaultPackage = forAllSystems (system: self.packages.${system}.cache-gc);
    packages = forAllSystems (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in rec {
      cache-gc = pkgs.callPackage ./package.nix { nix = pkgs.nixVersions.nix_2_19; src = self; };
      default = cache-gc;
    });

    hydraJobs = self.outputs.packages;

    devShells = forAllSystems (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = pkgs.mkShell {
          name = "cache-gc-dev";
          buildInputs = with pkgs; [ cargo rustfmt rustc ];
        };
      });

    nixosModules.cache-gc = { config, lib, pkgs, ... }: with lib;
      let
        cfg = config.services.cache-gc;
      in {
        options.services.cache-gc = {
          enable = mkEnableOption "automatic cache gc";
          path = mkOption {
            type = types.str;
            example = "/var/lib/hydra/cache-gc";
            description = mdDoc ''
              Path containing the binary cache to clean up.
            '';
          };
          package = mkOption {
            type = types.package;
            description = mdDoc ''
              The cache-gc package used for cleaning the cache.
            '';
            default = self.outputs.packages.${pkgs.system}.cache-gc;
            defaultText = "<cache-gc flake>.packages.<system>.cache-gc";
          };
          days = mkOption {
            default = 90;
            type = types.ints.positive;
            description = mdDoc ''
              Remove all paths that are older than the `n`
              days and not referenced by paths newer than `n`
              days with `n` being the value of this option.
            '';
          };
          frequency = mkOption {
            default = null;
            type = types.nullOr types.str;
            example = "daily";
            description = mdDoc ''
              When not `null`, a timer is added which periodically triggers
              the garbage collector. In that case, the option's value must represent
              the frequency of how often the service shuld be executed.

              Further information can be found in {manpage}`systemd.time(7)`.
            '';
          };
          owningUser = mkOption {
            type = types.str;
            example = "hydra-queue-runner";
            description = mdDoc ''
              User which owns the path [](#opt-services.cache-gc.path).
            '';
          };
        };
        config = mkIf cfg.enable {
          systemd.services.cache-gc = {
            description = "Nix binary cache garbage collector";
            environment.HOME = "/run/cache-gc";
            serviceConfig = {
              CapabilityBoundingSet = [ "" ];
              ExecStart = "${cfg.package}/bin/cache-gc --days ${toString cfg.days} --delete ${cfg.path}";
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
