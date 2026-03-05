{
  description = "Test mkFlake standalone mode (no flake-parts)";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    nlib.url = "path:../../..";
  };

  outputs = inputs:
    let
      system = "x86_64-linux";
      pkgs = inputs.nixpkgs.legacyPackages.${system};

      flakeOutputs = inputs.nlib.mkFlake {
        inherit inputs;
        modules = [
          ./libs/math.nix
        ];
      } {
        packages.${system}.default = pkgs.writeText "test" "double 5 = 10";
      };
    in
    let
      testApp = {
        type = "app";
        program = pkgs.writeShellApplication {
          name = "test-mkFlake-standalone";
          text = ''
            echo "=== mkFlake standalone test ==="
            echo ""
            echo "Testing lib.math.double..."
            result=$(nix eval .#lib.math.double --apply 'f: f 5')
            if [ "$result" = "10" ]; then
              echo "✓ lib.math.double 5 = $result"
            else
              echo "✗ lib.math.double 5 = $result (expected 10)"
              exit 1
            fi
            echo ""
            echo "Testing lib.math.triple..."
            result=$(nix eval .#lib.math.triple --apply 'f: f 3')
            if [ "$result" = "9" ]; then
              echo "✓ lib.math.triple 3 = $result"
            else
              echo "✗ lib.math.triple 3 = $result (expected 9)"
              exit 1
            fi
            echo ""
            echo "=== All mkFlake standalone tests passed! ==="
          '';
        } + "/bin/test-mkFlake-standalone";
      };
    in
    flakeOutputs // {
      apps.${system} = {
        test = testApp;
        default = testApp;
      };
    };
}
