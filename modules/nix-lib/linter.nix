# nix-lib.linter (flake-level)
#
# Exposes lib metadata for external linter tools.
# Accessible via `nix eval '.#linterMeta'`
#
{ lib, config, ... }:
let
  cfg = config.nix-lib;

  # Get flake-level lib metadata
  flakeLibsMeta = cfg._flakeLibsMeta or { };

  # Get collected metadata from all module systems
  collectedMeta = lib.mapAttrs (_: collector: collector config) (cfg.metaCollectors or { });

  # Merge all metadata
  allMeta = flakeLibsMeta // lib.foldl' (acc: meta: acc // meta) { } (lib.attrValues collectedMeta);

  # Transform metadata to JSON-serializable format
  serializableMeta = lib.mapAttrs (
    name: def: {
      inherit name;
      description = def.description or "";
      hasTests = (def.tests or { }) != { };
      testCount = builtins.length (builtins.attrNames (def.tests or { }));
      testNames = builtins.attrNames (def.tests or { });
      visible = def.visible or true;
      hasExample = (def.example or null) != null;
      file = def.file or null;
    }
  ) allMeta;
in
{
  # Expose metadata as flake output for linter
  config.flake.linterMeta = serializableMeta;
}
