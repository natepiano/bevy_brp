#!/usr/bin/env python3
"""Focused tests for extras_assert_png.py."""

from __future__ import annotations

import struct
import subprocess
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

import extras_assert_png


HELPER = Path(__file__).with_name("extras_assert_png.py")
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def png_chunk(chunk_type: bytes, data: bytes) -> bytes:
    crc = zlib.crc32(data, zlib.crc32(chunk_type)) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + chunk_type + data + struct.pack(">I", crc)


def write_png(
    path: Path,
    width: int,
    height: int,
    channels: int,
    pixels: bytes,
    *,
    include_iend: bool = True,
) -> None:
    color_type = 2 if channels == 3 else 6
    rows = b"".join(
        b"\x00" + pixels[row * width * channels : (row + 1) * width * channels]
        for row in range(height)
    )
    ihdr = struct.pack(">IIBBBBB", width, height, 8, color_type, 0, 0, 0)
    data = PNG_SIGNATURE + png_chunk(b"IHDR", ihdr)
    data += png_chunk(b"IDAT", zlib.compress(rows))
    if include_iend:
        data += png_chunk(b"IEND", b"")
    path.write_bytes(data)


def write_filtered_png(path: Path) -> None:
    decoded_rows = [
        bytes([10, 20, 30, 40, 50, 60]),
        bytes([15, 25, 35, 45, 55, 65]),
        bytes([20, 30, 40, 50, 60, 70]),
        bytes([25, 35, 45, 55, 65, 75]),
        bytes([30, 40, 50, 60, 70, 80]),
    ]
    encoded_rows = [
        b"\x00" + decoded_rows[0],
        bytes([1, 15, 25, 35, 30, 30, 30]),
        bytes([2, 5, 5, 5, 5, 5, 5]),
        bytes([3, 15, 20, 25, 18, 18, 18]),
        bytes([4, 5, 5, 5, 5, 5, 5]),
    ]
    ihdr = struct.pack(">IIBBBBB", 2, 5, 8, 2, 0, 0, 0)
    data = PNG_SIGNATURE + png_chunk(b"IHDR", ihdr)
    data += png_chunk(b"IDAT", zlib.compress(b"".join(encoded_rows)))
    data += png_chunk(b"IEND", b"")
    path.write_bytes(data)


def write_first_row_filtered_png(
    path: Path, channels: int, filter_type: int, decoded: bytes
) -> None:
    encoded = bytearray()
    for column, value in enumerate(decoded):
        left = decoded[column - channels] if column >= channels else 0
        if filter_type in (1, 4):
            predictor = left
        elif filter_type == 2:
            predictor = 0
        else:
            predictor = left // 2
        encoded.append((value - predictor) & 0xFF)

    width = len(decoded) // channels
    color_type = 2 if channels == 3 else 6
    ihdr = struct.pack(">IIBBBBB", width, 1, 8, color_type, 0, 0, 0)
    data = PNG_SIGNATURE + png_chunk(b"IHDR", ihdr)
    data += png_chunk(b"IDAT", zlib.compress(bytes([filter_type]) + encoded))
    data += png_chunk(b"IEND", b"")
    path.write_bytes(data)


def write_extreme_dimensions_png(path: Path) -> None:
    ihdr = struct.pack(">IIBBBBB", 0xFFFFFFFF, 0xFFFFFFFF, 8, 6, 0, 0, 0)
    data = PNG_SIGNATURE + png_chunk(b"IHDR", ihdr)
    data += png_chunk(b"IDAT", zlib.compress(b""))
    data += png_chunk(b"IEND", b"")
    path.write_bytes(data)


class ExtrasAssertPngTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def run_helper(self, *arguments: object) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(HELPER), *(str(argument) for argument in arguments)],
            check=False,
            capture_output=True,
            text=True,
        )

    def test_rgb_and_rgba_dimensions_decode(self) -> None:
        rgb = self.directory / "rgb.png"
        rgba = self.directory / "rgba.png"
        write_png(rgb, 2, 1, 3, bytes([255, 0, 0, 0, 255, 0]))
        write_png(rgba, 1, 2, 4, bytes([0, 0, 255, 128, 255, 255, 0, 255]))

        rgb_result = self.run_helper("dimensions", rgb, 2, 1)
        rgba_result = self.run_helper("dimensions", rgba, 1, 2)

        self.assertEqual(rgb_result.returncode, 0, rgb_result.stderr)
        self.assertIn("2x1 RGB", rgb_result.stdout)
        self.assertEqual(rgba_result.returncode, 0, rgba_result.stderr)
        self.assertIn("1x2 RGBA", rgba_result.stdout)
        self.assertEqual(extras_assert_png.decode_png(rgba).pixel(0, 0), (0, 0, 255, 128))

    def test_all_png_row_filters_decode(self) -> None:
        path = self.directory / "filters.png"
        write_filtered_png(path)

        image = extras_assert_png.decode_png(path)

        self.assertEqual(image.pixel(0, 0), (10, 20, 30, 255))
        self.assertEqual(image.pixel(1, 4), (60, 70, 80, 255))

    def test_first_row_filters_decode_rgb_and_rgba(self) -> None:
        for channels in (3, 4):
            decoded = bytes(range(10, 10 + 2 * channels))
            for filter_type in range(1, 5):
                with self.subTest(channels=channels, filter_type=filter_type):
                    path = self.directory / f"first_row_{channels}_{filter_type}.png"
                    write_first_row_filtered_png(
                        path, channels, filter_type, decoded
                    )

                    image = extras_assert_png.decode_png(path)

                    self.assertEqual(image.pixels, decoded)

    def test_extreme_dimensions_fail_cleanly_at_size_limit(self) -> None:
        path = self.directory / "extreme_dimensions.png"
        write_extreme_dimensions_png(path)

        result = self.run_helper("dimensions", path, 1, 1)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("helper limit", result.stderr)
        self.assertIn("64 MiB", result.stderr)
        self.assertNotIn("Traceback", result.stdout + result.stderr)

    def test_expected_present_and_absent_paths(self) -> None:
        present = self.directory / "present.png"
        absent = self.directory / "absent.png"
        write_png(present, 1, 1, 3, bytes([1, 2, 3]))

        self.assertEqual(self.run_helper("present", present).returncode, 0)
        self.assertNotEqual(self.run_helper("present", absent).returncode, 0)
        self.assertEqual(self.run_helper("absent", absent).returncode, 0)
        self.assertNotEqual(self.run_helper("absent", present).returncode, 0)

    def test_malformed_signature_and_crc_fail(self) -> None:
        malformed = self.directory / "malformed.png"
        malformed.write_bytes(b"not a png")
        bad_crc = self.directory / "bad_crc.png"
        write_png(bad_crc, 1, 1, 3, bytes([1, 2, 3]))
        contents = bytearray(bad_crc.read_bytes())
        contents[-1] ^= 1
        bad_crc.write_bytes(contents)

        self.assertNotEqual(self.run_helper("dimensions", malformed, 1, 1).returncode, 0)
        self.assertNotEqual(self.run_helper("dimensions", bad_crc, 1, 1).returncode, 0)

    def test_incomplete_png_fails(self) -> None:
        incomplete = self.directory / "incomplete.png"
        write_png(incomplete, 1, 1, 3, bytes([1, 2, 3]), include_iend=False)

        result = self.run_helper("dimensions", incomplete, 1, 1)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("IEND", result.stderr)

    def test_uniform_detection_accepts_only_multiple_colors(self) -> None:
        uniform = self.directory / "uniform.png"
        varied = self.directory / "varied.png"
        write_png(uniform, 2, 1, 3, bytes([4, 5, 6, 4, 5, 6]))
        write_png(varied, 2, 1, 3, bytes([4, 5, 6, 7, 8, 9]))

        self.assertNotEqual(self.run_helper("nonuniform", uniform).returncode, 0)
        self.assertEqual(self.run_helper("nonuniform", varied).returncode, 0)

    def test_marker_checks_rgb_rgba_and_mismatch(self) -> None:
        rgb = self.directory / "marker_rgb.png"
        rgba = self.directory / "marker_rgba.png"
        write_png(rgb, 1, 1, 3, bytes([10, 20, 30]))
        write_png(rgba, 1, 1, 4, bytes([10, 20, 30, 40]))

        self.assertEqual(
            self.run_helper("marker", rgb, 20, 30, 20, 30, 10, 20, 30).returncode,
            0,
        )
        self.assertEqual(
            self.run_helper(
                "marker", rgba, 20, 30, 20, 30, 10, 20, 30, 40
            ).returncode,
            0,
        )
        mismatch = self.run_helper("marker", rgb, 20, 30, 20, 30, 30, 20, 10)
        self.assertNotEqual(mismatch.returncode, 0)
        self.assertIn("expected", mismatch.stderr)

    def test_exact_crop_match_and_mismatch(self) -> None:
        reference = self.directory / "reference.png"
        crop = self.directory / "crop.png"
        mismatched = self.directory / "mismatched.png"
        reference_pixels = b"".join(bytes([value, 0, 0]) for value in range(1, 13))
        crop_pixels = bytes([6, 0, 0, 7, 0, 0, 10, 0, 0, 11, 0, 0])
        write_png(reference, 4, 3, 3, reference_pixels)
        write_png(crop, 2, 2, 3, crop_pixels)
        write_png(mismatched, 2, 2, 3, bytes([0]) + crop_pixels[1:])

        match = self.run_helper("crop", crop, reference, 11, 21, 10, 20, 2, 2)
        mismatch = self.run_helper(
            "crop", mismatched, reference, 11, 21, 10, 20, 2, 2
        )

        self.assertEqual(match.returncode, 0, match.stderr)
        self.assertNotEqual(mismatch.returncode, 0)
        self.assertIn("crop mismatch", mismatch.stderr)


if __name__ == "__main__":
    unittest.main()
