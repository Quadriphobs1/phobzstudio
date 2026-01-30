{
  description = "Phobz Audio Visualizer - GPU-accelerated music visualization";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
        };

        # Rust toolchain with components
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };

        # Python 3.14 with packages
        python = pkgs.python314;
        pythonPackages = pkgs.python314Packages;

        # Common build inputs
        commonBuildInputs = with pkgs; [
          # Rust
          rustToolchain
          cargo-watch
          cargo-expand

          # Python
          python
          pythonPackages.pip
          pythonPackages.virtualenv
          pythonPackages.numpy
          pythonPackages.pillow
          pythonPackages.cffi
          ruff

          # Build tools
          pkg-config
          cmake
          ninja
          maturin
          just

          # FFmpeg 8.x (full build for ProRes support)
          ffmpeg_8-full

          # Bun (for future Remotion support)
          bun

          # Git
          git
        ];

        # macOS-specific dependencies
        darwinBuildInputs = with pkgs; [
          apple-sdk_15
          libiconv
        ];

        # Linux-specific dependencies
        linuxBuildInputs = with pkgs; [
          vulkan-loader
          vulkan-headers
          vulkan-tools
          vulkan-validation-layers
          alsa-lib
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          wayland
          libxkbcommon
        ];

        # Platform-specific inputs
        platformInputs = if pkgs.stdenv.isDarwin then darwinBuildInputs else linuxBuildInputs;

        # Environment variables for FFmpeg linking
        ffmpegEnv = {
          FFMPEG_DIR = "${pkgs.ffmpeg_8-full}";
          FFMPEG_INCLUDE_DIR = "${pkgs.ffmpeg_8-full.dev}/include";
          FFMPEG_LIB_DIR = "${pkgs.ffmpeg_8-full.lib}/lib";
          PKG_CONFIG_PATH = "${pkgs.ffmpeg_8-full.dev}/lib/pkgconfig";
        };

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = commonBuildInputs ++ platformInputs;

          shellHook = ''
            # FFmpeg environment
            export FFMPEG_DIR="${pkgs.ffmpeg_8-full}"
            export FFMPEG_INCLUDE_DIR="${pkgs.ffmpeg_8-full.dev}/include"
            export FFMPEG_LIB_DIR="${pkgs.ffmpeg_8-full.lib}/lib"
            export PKG_CONFIG_PATH="${pkgs.ffmpeg_8-full.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

            # Rust environment
            export RUST_BACKTRACE=1
            export RUST_LOG=info

            # Python virtual environment
            if [ ! -d .venv ]; then
              python -m venv .venv
            fi
            source .venv/bin/activate

            # Install Python dev dependencies if not present
            if ! python -c "import typer" 2>/dev/null; then
              pip install --quiet typer rich pydantic pytest
            fi

            echo "Phobz Visualizer Development Environment"
            echo "========================================="
            echo "Rust:   $(rustc --version)"
            echo "Python: $(python --version)"
            echo "FFmpeg: $(ffmpeg -version 2>&1 | head -n1)"
            echo "Bun:    $(bun --version)"
            echo ""
            echo "Commands:"
            echo "  just build    - Build Rust + Python bindings"
            echo "  just test     - Run all tests"
            echo "  just dev      - Auto-rebuild on changes"
            echo ""
          '';

          # macOS-specific environment
          MACOSX_DEPLOYMENT_TARGET = if pkgs.stdenv.isDarwin then "11.0" else null;

          # For wgpu on macOS
          LIBCLANG_PATH = if pkgs.stdenv.isDarwin then "${pkgs.llvmPackages.libclang.lib}/lib" else null;
        };
      }
    );
}
