# treefmt configuration for code formatting and linting
# This module is loaded only in the dev partition
{ inputs, ... }:
{
  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem = _: {
    treefmt = {
      projectRootFile = "flake.nix";

      programs = {
        # Nix formatting
        nixfmt.enable = true;

        # Dead code detection
        deadnix.enable = true;

        # Static analysis
        statix.enable = true;
      };
    };
  };
}
