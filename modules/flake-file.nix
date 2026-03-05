# flake-file configuration for auto-generating flake.nix
{ inputs, lib, ... }:
{
  imports = [
    inputs.flake-file.flakeModules.dendritic
  ];

  flake-file = {
    description = "nix-lib - Nix library module with tested, typed, documented functions";

    # Custom outputs to expose lib utilities for specialArgs injection
    # These are available BEFORE module evaluation, enabling use in options phase
    outputs = lib.mkForce ''
      inputs:
        let
          lib = inputs.nixpkgs.lib;
          nlibLib = import ./modules/nix-lib/_lib { inherit lib; };
        in
        inputs.flake-parts.lib.mkFlake { inherit inputs; } (inputs.import-tree ./modules)
        // { inherit (nlibLib) mkFlake evalLibModules mkSpecialArgsLib mkLib mkAdapter withLib; }
    '';

    inputs = {
      nix-unit = {
        url = "github:nix-community/nix-unit";
        inputs.nixpkgs.follows = "nixpkgs";
        inputs.flake-parts.follows = "flake-parts";
      };

      treefmt-nix = {
        url = "github:numtide/treefmt-nix";
        inputs.nixpkgs.follows = "nixpkgs";
      };

      devour-flake = {
        url = "github:srid/devour-flake";
        flake = false;
      };
      get-flake.url = "github:ursi/get-flake";
    };
  };
}
