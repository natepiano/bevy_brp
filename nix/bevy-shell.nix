# Why this exists: cargo builds a Bevy crate with rustup's toolchain, outside
# nix, and on NixOS there is no global lib directory for it to find alsa,
# vulkan or X11 in. This shell hands those over -- headers and pkg-config at
# build time, LD_LIBRARY_PATH for what the binary dlopens at run time -- and
# nothing else: no Rust toolchain, no cargo tools. On macOS it supplies
# pkg-config alone, because the frameworks Bevy links there come from the SDK.
#
# It is the set every Bevy crate needs. A caller passes `extraLinuxLibraries`
# for whatever its own crates add on top, and `flake.nix` is where that goes.
#
# VENDORED ON PURPOSE. Byte-identical copies live in cargo-liner, nateroids,
# bevy_brp and hana, so each clone builds from nixpkgs alone with no flake
# input pointing at another repo. The cost is that a change here is four
# commits -- pay it, and keep the copies identical rather than letting them
# drift (decided 2026-09-08, after the mkShellNoCC fix below cost exactly that).
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
  # Each entry says what opens it, so the list can be edited with confidence.
  linuxLibraries =
    with pkgs;
    [
      alsa-lib # audio (cpal)
      systemdLibs # libudev: gamepad input (gilrs); bindgen reads its headers
      vulkan-loader # graphics (wgpu), opened at run time
      libGL # the GL fallback (wgpu, WGPU_BACKEND=gl in CI)
      wayland # windowing (winit)
      libxkbcommon # keyboard (winit)
      libx11 # the X11 fallback (winit)
      libxcursor # the X11 fallback (winit)
      libxi # the X11 fallback (winit)
      libxrandr # the X11 fallback (winit)
    ]
    ++ extraLinuxLibraries;
in
mkShell (
  {
    nativeBuildInputs = [
      # Its setup hook sets PKG_CONFIG_PATH from buildInputs.
      pkgs.pkg-config
    ]
    ++ lib.optionals stdenv.isLinux [
      # Sets LIBCLANG_PATH and BINDGEN_EXTRA_CLANG_ARGS for the -sys crates
      # that run bindgen (libudev-sys).
      pkgs.rustPlatform.bindgenHook
      # Puts ld.mold on PATH so CI's `-C link-arg=-fuse-ld=mold` resolves
      # through the nix cc wrapper.
      pkgs.mold-wrapped
    ];
    buildInputs = lib.optionals stdenv.isLinux linuxLibraries;
  }
  // lib.optionalAttrs stdenv.isLinux {
    # Set outright rather than appended to: the libraries above are the complete
    # set the binaries open at run time, and a value inherited from the host
    # would put its own vulkan-loader or libGL first.
    LD_LIBRARY_PATH = lib.makeLibraryPath linuxLibraries;
  }
)
