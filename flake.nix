# The development shell for this crate: the native libraries Bevy links or
# opens at run time on Linux, declared once here and entered by `nix develop`,
# by direnv through .envrc, and by CI through `nix develop -c`. The Rust
# toolchain is not in it -- rustup's is used everywhere, as before.
{
  description = "bevy_brp development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = import ./nix/bevy-shell.nix { inherit pkgs; };
      }
    );
}
