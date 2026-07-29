{
  description = "A ready to go Wayland status bar for Hyprland";

  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { crane, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          
          craneLib = crane.mkLib pkgs;

          buildInputs = with pkgs; [
            rust-bin.stable.latest.default
            rustPlatform.bindgenHook
            pkg-config
            libxkbcommon
            libGL
            pipewire
            libpulseaudio
            wayland
            vulkan-loader
            udev
          ];

          runtimeDependencies = with pkgs; [
            libpulseaudio
            wayland
            mesa
            vulkan-loader
            libGL
            libglvnd
          ];
            
          ldLibraryPath = pkgs.lib.makeLibraryPath runtimeDependencies;
        in
        {
          # `nix build` and `nix run`
          defaultPackage = craneLib.buildPackage {
            src = ./.;

            nativeBuildInputs = with pkgs; [
              makeWrapper
              pkg-config
              autoPatchelfHook # Add runtimeDependencies to rpath
            ];

            inherit buildInputs runtimeDependencies ldLibraryPath;

            postInstall = ''
              # The crate builds as 'hydebar-app'; the bar is installed under
              # the name everything else refers to it by.
              if [ -f "$out/bin/hydebar-app" ]; then
                mv "$out/bin/hydebar-app" "$out/bin/hydebar"
              fi
              wrapProgram "$out/bin/hydebar" --prefix LD_LIBRARY_PATH : "${ldLibraryPath}"

              # The theme switch script, in the shared-data spot the bar
              # searches relative to its own binary.
              install -Dm755 ${./scripts/theme-switch} "$out/share/hydebar/scripts/theme-switch"
            '';
          };

          # `nix develop`
          devShells.default = pkgs.mkShell {
            inherit buildInputs ldLibraryPath;

            LD_LIBRARY_PATH = ldLibraryPath;
          };
        }
      );
}

