# A Bevy crate's development shell. Everything native the crate needs on
# Linux and nothing else: no Rust toolchain (rustup's, outside nix, as on the
# machines), no cargo tools.
#
# On macOS the shell supplies pkg-config alone; the system frameworks Bevy
# links there come from the SDK, not from nix. See mkShell in the let below.
#
# WHAT EACH LIBRARY IS FOR, so that the list can be edited with confidence:
#   alsa-lib      audio (cpal)
#   systemdLibs   libudev, gamepad input (gilrs); bindgen reads its headers
#   vulkan-loader graphics (wgpu), opened at run time
#   libGL         the GL fallback (wgpu, WGPU_BACKEND=gl in CI)
#   wayland, libxkbcommon              windowing and keyboard (winit)
#   libx11, libxcursor, libxi, libxrandr   the X11 fallback (winit)
#
# rustPlatform.bindgenHook sets LIBCLANG_PATH and BINDGEN_EXTRA_CLANG_ARGS
# for the -sys crates that run bindgen (libudev-sys); mold-wrapped puts
# ld.mold on PATH so CI's `-C link-arg=-fuse-ld=mold` resolves through the
# nix cc wrapper; pkg-config's hook sets
# PKG_CONFIG_PATH from buildInputs. LD_LIBRARY_PATH is set outright rather
# than appended to: the libraries here are the complete set the binaries
# open at run time, and a value inherited from the host would put its own
# vulkan-loader or libGL first.
{
  pkgs,
  extraLinuxLibraries ? [ ],
}:
let
  inherit (pkgs) lib stdenv;
  # Darwin gets a shell with no C compiler in it. pkgs.mkShell is
  # stdenv.mkDerivation underneath, so it brings nixpkgs' cc, and on darwin that
  # cc depends on apple-sdk-14.4 and exports SDKROOT and DEVELOPER_DIR. A build
  # script that shells out to Swift Package Manager then compiles its
  # Package.swift with the Command Line Tools swiftc -- nix has no swiftc --
  # against that older SDK, and swift refuses the pairing outright ("this SDK is
  # not supported by the compiler"). Nothing here wants a C compiler on macOS.
  # Linux keeps mkShell: rustc links through the cc wrapper there, and that is
  # how CI's -fuse-ld=mold resolves.
  mkShell = if stdenv.isDarwin then pkgs.mkShellNoCC else pkgs.mkShell;
  linuxLibraries =
    with pkgs;
    [
      alsa-lib
      systemdLibs
      vulkan-loader
      libGL
      wayland
      libxkbcommon
      libx11
      libxcursor
      libxi
      libxrandr
    ]
    ++ extraLinuxLibraries;
in
mkShell (
  {
    nativeBuildInputs = [
      pkgs.pkg-config
    ]
    ++ lib.optionals stdenv.isLinux [
      pkgs.rustPlatform.bindgenHook
      pkgs.mold-wrapped
    ];
    buildInputs = lib.optionals stdenv.isLinux linuxLibraries;
  }
  // lib.optionalAttrs stdenv.isLinux {
    LD_LIBRARY_PATH = lib.makeLibraryPath linuxLibraries;
  }
)
