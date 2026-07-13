#!/usr/bin/env python3
"""Read-only assertions for terminal screenshot PNG files."""

from __future__ import annotations

import argparse
import os
import struct
import sys
import zlib
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Sequence


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
COLOR_TYPE_RGB = 2
COLOR_TYPE_RGBA = 6
FILTER_BYTES_PER_ROW = 1
MAX_CHANNEL_VALUE = 255
# This integration helper only decodes small screenshots. Bound the total
# decoded scanline data so untrusted IHDR dimensions cannot request huge buffers.
MAX_DECODED_SCANLINE_BYTES = 64 * 1024 * 1024


class PngError(ValueError):
    """Raised when a PNG does not match the supported complete encoding."""


@dataclass(frozen=True)
class DecodedPng:
    """Decoded 8-bit RGB or RGBA pixels stored in row-major order."""

    width: int
    height: int
    channels: int
    pixels: bytes

    @property
    def color_name(self) -> str:
        """Return the supported PNG color name."""
        return "RGB" if self.channels == 3 else "RGBA"

    def pixel(self, x: int, y: int) -> tuple[int, int, int, int]:
        """Return one pixel normalized to RGBA."""
        if not 0 <= x < self.width or not 0 <= y < self.height:
            raise PngError(
                f"pixel ({x}, {y}) is outside {self.width}x{self.height} image"
            )
        offset = (y * self.width + x) * self.channels
        values = self.pixels[offset : offset + self.channels]
        if self.channels == 3:
            return values[0], values[1], values[2], MAX_CHANNEL_VALUE
        return values[0], values[1], values[2], values[3]

    def is_uniform(self) -> bool:
        """Return whether every decoded pixel has the same channel values."""
        first = self.pixels[: self.channels]
        return all(
            self.pixels[offset : offset + self.channels] == first
            for offset in range(0, len(self.pixels), self.channels)
        )


@dataclass(frozen=True)
class PngHeader:
    """Validated IHDR fields needed by the decoder."""

    width: int
    height: int
    channels: int


def decode_png(path: Path) -> DecodedPng:
    """Decode a complete non-interlaced 8-bit RGB or RGBA PNG."""
    data = path.read_bytes()
    if not data.startswith(PNG_SIGNATURE):
        raise PngError(f"{path} has an invalid PNG signature")

    header: PngHeader | None = None
    idat_parts: list[bytes] = []
    idat_closed = False
    saw_iend = False
    offset = len(PNG_SIGNATURE)

    while offset < len(data):
        if len(data) - offset < 12:
            raise PngError(f"{path} ends inside a PNG chunk")
        length = struct.unpack_from(">I", data, offset)[0]
        chunk_type = data[offset + 4 : offset + 8]
        chunk_end = offset + 12 + length
        if chunk_end > len(data):
            raise PngError(f"{path} ends inside {chunk_type!r} chunk data")

        chunk_data = data[offset + 8 : offset + 8 + length]
        expected_crc = struct.unpack_from(">I", data, offset + 8 + length)[0]
        actual_crc = zlib.crc32(chunk_data, zlib.crc32(chunk_type)) & 0xFFFFFFFF
        if actual_crc != expected_crc:
            raise PngError(f"{path} has an invalid {chunk_type!r} chunk CRC")
        offset = chunk_end

        if header is None and chunk_type != b"IHDR":
            raise PngError(f"{path} does not begin with an IHDR chunk")
        if chunk_type == b"IHDR":
            if header is not None or idat_parts:
                raise PngError(f"{path} has a misplaced or duplicate IHDR chunk")
            header = _decode_header(path, chunk_data)
        elif chunk_type == b"IDAT":
            if header is None or idat_closed:
                raise PngError(f"{path} has a misplaced IDAT chunk")
            idat_parts.append(chunk_data)
        elif chunk_type == b"IEND":
            if chunk_data or header is None or not idat_parts:
                raise PngError(f"{path} has an invalid IEND chunk")
            saw_iend = True
            if offset != len(data):
                raise PngError(f"{path} has data after IEND")
            break
        else:
            if idat_parts:
                idat_closed = True
            if chunk_type and chunk_type[0] & 0x20 == 0:
                raise PngError(f"{path} uses unsupported critical chunk {chunk_type!r}")

    if header is None:
        raise PngError(f"{path} has no IHDR chunk")
    if not idat_parts:
        raise PngError(f"{path} has no IDAT chunk")
    if not saw_iend:
        raise PngError(f"{path} has no complete IEND chunk")

    stride, expected_size = _checked_decoded_sizes(path, header)
    decompressed = _decompress_exact(path, b"".join(idat_parts), expected_size)
    pixels = _unfilter(path, decompressed, header, stride)
    return DecodedPng(header.width, header.height, header.channels, pixels)


def _decode_header(path: Path, data: bytes) -> PngHeader:
    if len(data) != 13:
        raise PngError(f"{path} has an invalid IHDR length")
    width, height, bit_depth, color_type, compression, row_filter, interlace = (
        struct.unpack(">IIBBBBB", data)
    )
    if width == 0 or height == 0:
        raise PngError(f"{path} has zero PNG dimensions")
    if bit_depth != 8 or color_type not in (COLOR_TYPE_RGB, COLOR_TYPE_RGBA):
        raise PngError(f"{path} must use 8-bit RGB or RGBA pixels")
    if compression != 0 or row_filter != 0 or interlace != 0:
        raise PngError(
            f"{path} must use standard compression/filtering and no interlace"
        )
    channels = 3 if color_type == COLOR_TYPE_RGB else 4
    return PngHeader(width, height, channels)


def _checked_decoded_sizes(path: Path, header: PngHeader) -> tuple[int, int]:
    """Return bounded decoder sizes before zlib or unfilter allocations."""
    stride = header.width * header.channels
    scanline_size = header.height * (stride + FILTER_BYTES_PER_ROW)
    if scanline_size > MAX_DECODED_SCANLINE_BYTES:
        raise PngError(
            f"{path} decoded scanlines require {scanline_size} bytes; "
            f"helper limit is {MAX_DECODED_SCANLINE_BYTES} bytes (64 MiB)"
        )
    return stride, scanline_size


def _decompress_exact(path: Path, compressed: bytes, expected_size: int) -> bytes:
    decoder = zlib.decompressobj()
    try:
        decompressed = decoder.decompress(compressed, expected_size + 1)
    except zlib.error as error:
        raise PngError(f"{path} has invalid compressed PNG data: {error}") from error
    if (
        len(decompressed) != expected_size
        or not decoder.eof
        or decoder.unconsumed_tail
        or decoder.unused_data
    ):
        raise PngError(
            f"{path} decompressed to {len(decompressed)} bytes; expected {expected_size}"
        )
    return decompressed


def _unfilter(path: Path, data: bytes, header: PngHeader, stride: int) -> bytes:
    output = bytearray()
    previous = bytearray(stride)
    offset = 0
    for row_index in range(header.height):
        filter_type = data[offset]
        offset += FILTER_BYTES_PER_ROW
        encoded = data[offset : offset + stride]
        offset += stride
        row = bytearray(stride)
        for column, value in enumerate(encoded):
            left = row[column - header.channels] if column >= header.channels else 0
            above = previous[column]
            upper_left = (
                previous[column - header.channels]
                if column >= header.channels
                else 0
            )
            predictor = _filter_predictor(filter_type, left, above, upper_left)
            if predictor is None:
                raise PngError(
                    f"{path} row {row_index} uses invalid PNG filter {filter_type}"
                )
            row[column] = (value + predictor) & MAX_CHANNEL_VALUE
        output.extend(row)
        previous = row
    return bytes(output)


def _filter_predictor(
    filter_type: int, left: int, above: int, upper_left: int
) -> int | None:
    if filter_type == 0:
        return 0
    if filter_type == 1:
        return left
    if filter_type == 2:
        return above
    if filter_type == 3:
        return (left + above) // 2
    if filter_type == 4:
        return _paeth(left, above, upper_left)
    return None


def _paeth(left: int, above: int, upper_left: int) -> int:
    estimate = left + above - upper_left
    left_distance = abs(estimate - left)
    above_distance = abs(estimate - above)
    upper_left_distance = abs(estimate - upper_left)
    if left_distance <= above_distance and left_distance <= upper_left_distance:
        return left
    if above_distance <= upper_left_distance:
        return above
    return upper_left


def assert_absent(path: Path) -> None:
    """Assert a destination does not exist before a screenshot call."""
    if os.path.lexists(path):
        raise PngError(f"expected {path} to be absent")
    print(f"absent: {path}")


def assert_present(path: Path) -> None:
    """Assert a terminal screenshot path is an existing regular file."""
    if not path.is_file():
        raise PngError(f"expected {path} to be a present file")
    print(f"present: {path}")


def assert_dimensions(path: Path, width: int, height: int) -> None:
    """Assert dimensions and report the decoded PNG format."""
    image = decode_png(path)
    if (image.width, image.height) != (width, height):
        raise PngError(
            f"{path} is {image.width}x{image.height}; expected {width}x{height}"
        )
    print(f"dimensions: {path}: {image.width}x{image.height} {image.color_name}")


def assert_nonuniform(path: Path) -> None:
    """Assert the decoded image contains more than one pixel value."""
    image = decode_png(path)
    if image.is_uniform():
        raise PngError(f"{path} is uniform with pixel {image.pixel(0, 0)}")
    print(f"nonuniform: {path}")


def assert_marker(
    path: Path,
    image_x: int,
    image_y: int,
    marker_x: int,
    marker_y: int,
    expected: Sequence[int],
) -> None:
    """Assert one target-space marker pixel."""
    image = decode_png(path)
    expected_rgba = tuple(expected) if len(expected) == 4 else (*expected, 255)
    local_x = marker_x - image_x
    local_y = marker_y - image_y
    actual = image.pixel(local_x, local_y)
    if actual != expected_rgba:
        raise PngError(
            f"{path} target pixel ({marker_x}, {marker_y}) is {actual}; "
            f"expected {expected_rgba}"
        )
    print(f"marker: {path}: target ({marker_x}, {marker_y})={actual}")


def assert_crop(
    crop_path: Path,
    reference_path: Path,
    target_x: int,
    target_y: int,
    reference_x: int,
    reference_y: int,
    width: int,
    height: int,
) -> None:
    """Assert every crop pixel equals its reference rectangle pixel."""
    if width <= 0 or height <= 0 or min(target_x, target_y, reference_x, reference_y) < 0:
        raise PngError("crop rectangle must have nonnegative origin and positive dimensions")
    crop = decode_png(crop_path)
    reference = decode_png(reference_path)
    if (crop.width, crop.height) != (width, height):
        raise PngError(
            f"{crop_path} is {crop.width}x{crop.height}; expected {width}x{height}"
        )
    relative_x = target_x - reference_x
    relative_y = target_y - reference_y
    if (
        relative_x < 0
        or relative_y < 0
        or relative_x + width > reference.width
        or relative_y + height > reference.height
    ):
        raise PngError(
            f"target crop ({target_x}, {target_y}, {width}, {height}) exceeds reference "
            f"({reference_x}, {reference_y}, {reference.width}, {reference.height})"
        )

    for local_y in range(height):
        for local_x in range(width):
            actual = crop.pixel(local_x, local_y)
            expected = reference.pixel(relative_x + local_x, relative_y + local_y)
            if actual != expected:
                raise PngError(
                    f"crop mismatch at ({local_x}, {local_y}): {actual}; "
                    f"reference target ({reference_x + relative_x + local_x}, "
                    f"{reference_y + relative_y + local_y}) is {expected}"
                )
    print(
        f"crop: {crop_path} target rectangle ({target_x}, {target_y}, {width}, {height}) "
        f"equals {reference_path} origin ({reference_x}, {reference_y})"
    )


def _byte_value(value: str) -> int:
    parsed = int(value)
    if not 0 <= parsed <= MAX_CHANNEL_VALUE:
        raise argparse.ArgumentTypeError(f"channel must be 0..{MAX_CHANNEL_VALUE}")
    return parsed


def _positive_integer(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be positive")
    return parsed


def _run_absent(args: argparse.Namespace) -> None:
    assert_absent(args.path)


def _run_present(args: argparse.Namespace) -> None:
    assert_present(args.path)


def _run_dimensions(args: argparse.Namespace) -> None:
    assert_dimensions(args.path, args.width, args.height)


def _run_nonuniform(args: argparse.Namespace) -> None:
    assert_nonuniform(args.path)


def _run_marker(args: argparse.Namespace) -> None:
    expected = (args.red, args.green, args.blue)
    if args.alpha is not None:
        expected = (*expected, args.alpha)
    assert_marker(
        args.path,
        args.image_x,
        args.image_y,
        args.marker_x,
        args.marker_y,
        expected,
    )


def _run_crop(args: argparse.Namespace) -> None:
    assert_crop(
        args.crop,
        args.reference,
        args.crop_x,
        args.crop_y,
        args.reference_x,
        args.reference_y,
        args.width,
        args.height,
    )


def _path_argument(parser: argparse.ArgumentParser, name: str = "path") -> None:
    parser.add_argument(name, type=Path)


def build_parser() -> argparse.ArgumentParser:
    """Build the exact read-only command forms used by integration tests."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="mode", required=True)

    absent = subparsers.add_parser("absent", help="assert path absence before a call")
    _path_argument(absent)
    absent.set_defaults(action=_run_absent)

    present = subparsers.add_parser("present", help="assert a returned path is a file")
    _path_argument(present)
    present.set_defaults(action=_run_present)

    dimensions = subparsers.add_parser(
        "dimensions", help="decode and assert PNG dimensions"
    )
    _path_argument(dimensions)
    dimensions.add_argument("width", type=_positive_integer)
    dimensions.add_argument("height", type=_positive_integer)
    dimensions.set_defaults(action=_run_dimensions)

    nonuniform = subparsers.add_parser(
        "nonuniform", help="reject a single-color PNG"
    )
    _path_argument(nonuniform)
    nonuniform.set_defaults(action=_run_nonuniform)

    marker = subparsers.add_parser("marker", help="assert one RGB/RGBA pixel")
    _path_argument(marker)
    marker.add_argument("image_x", type=int)
    marker.add_argument("image_y", type=int)
    marker.add_argument("marker_x", type=int)
    marker.add_argument("marker_y", type=int)
    marker.add_argument("red", type=_byte_value)
    marker.add_argument("green", type=_byte_value)
    marker.add_argument("blue", type=_byte_value)
    marker.add_argument("alpha", type=_byte_value, nargs="?")
    marker.set_defaults(action=_run_marker)

    crop = subparsers.add_parser(
        "crop", help="compare a PNG with a reference rectangle"
    )
    _path_argument(crop, "crop")
    _path_argument(crop, "reference")
    crop.add_argument("crop_x", type=int)
    crop.add_argument("crop_y", type=int)
    crop.add_argument("reference_x", type=int)
    crop.add_argument("reference_y", type=int)
    crop.add_argument("width", type=_positive_integer)
    crop.add_argument("height", type=_positive_integer)
    crop.set_defaults(action=_run_crop)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    """Run one assertion mode and return a process exit code."""
    args = build_parser().parse_args(argv)
    action: Callable[[argparse.Namespace], None] = args.action
    try:
        action(args)
    except (OSError, PngError) as error:
        print(f"PNG assertion failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
