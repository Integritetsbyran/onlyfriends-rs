{
  description = "OnlyFriends development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Native dependencies for dioxus-native/Blitz
        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
          rustToolchain
          python3  # Required for stylo build
        ];

        # Runtime/linking dependencies
        buildInputs = with pkgs; [
          # OpenSSL for reqwest
          openssl

          # Vulkan for GPU rendering (Blitz)
          vulkan-loader
          vulkan-headers
          vulkan-tools
          vulkan-validation-layers

          # X11 windowing
          libx11
          libxcursor
          libxrandr
          libxi
          libxcb

          # Wayland windowing
          wayland
          libxkbcommon

          # Font rendering
          fontconfig
          freetype

          # Additional graphics dependencies
          libGL
          mesa

          # GLib/GObject (required by some GTK-based deps)
          glib
          gdk-pixbuf

          # GTK3 stack (required by dioxus-native)
          gtk3
          atk
          cairo
          pango

          # WebKitGTK (required by dioxus webview)
          webkitgtk_4_1
          libsoup_3

          # xdotool for input simulation
          xdotool

          # Dioxus CLI
          dioxus-cli

          # Fast linker
          mold
          clang
        ];

        # Libraries to find at runtime
        libPath = pkgs.lib.makeLibraryPath buildInputs;
      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH="${libPath}:$LD_LIBRARY_PATH"
            export VULKAN_SDK="${pkgs.vulkan-headers}"
            export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d"
          '';

          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
        };
      });
}
