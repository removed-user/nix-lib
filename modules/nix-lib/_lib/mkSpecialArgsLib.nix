# mkSpecialArgsLib - Create libs for specialArgs injection
#
# Extract functions from lib definitions for early access via flake-parts specialArgs.
# Functions defined this way are available during ALL module evaluation phases
# (imports, options, config) - not just config phase.
#
# Usage in consumer flake.nix:
#   outputs = inputs@{ flake-parts, nlib, nixpkgs, ... }:
#     let
#       myLib = nlib.lib.mkSpecialArgsLib {
#         libs = {
#           double = {
#             fn = x: x * 2;
#             description = "Double a number";
#           };
#           utils.format = {
#             fn = s: "formatted: ${s}";
#             description = "Format string";
#           };
#         };
#       };
#     in
#     flake-parts.lib.mkFlake {
#       inherit inputs;
#       specialArgs = { inherit myLib; };  # Available in all phases!
#     } { ... };
#
# Then in any module:
#   { lib, myLib, ... }:
#   {
#     options.foo = myLib.double 5;  # Works in options phase!
#   }
#
{ lib }:
let
  libDefTypeModule = import ./libDefType.nix { inherit lib; };
  inherit (libDefTypeModule) flattenLibs unflattenFns extractFnsFlat;
in
{ libs }:
let
  # Flatten nested definitions to dotted names
  flatLibDefs = flattenLibs "" libs;

  # Extract only the fn attribute from each definition
  extractedFns = extractFnsFlat flatLibDefs;

  # Rebuild nested structure
  nestedFns = unflattenFns extractedFns;
in
nestedFns
