"""Encode 480x272 WebKit RGBA captures as PNGs and paired screen previews."""
import argparse
from pathlib import Path
import struct
import zlib


def write_png(path, rgba, width, height):
    if len(rgba) != width * height * 4:
        raise ValueError(f'{path}: unexpected RGBA dimensions')
    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body) & 0xffffffff)
    scanlines = b''.join(b'\0' + rgba[y * width * 4:(y + 1) * width * 4] for y in range(height))
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)) +
                     chunk(b'IDAT', zlib.compress(scanlines)) + chunk(b'IEND', b''))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path, help='A check_g1.py evidence directory')
    args = parser.parse_args()
    for raw in args.folder.glob('*/*.rgba'):
        write_png(raw.with_suffix('.png'), raw.read_bytes(), 480, 272)
    for left_file in args.folder.glob('*/left.rgba'):
        right_file = left_file.with_name('right.rgba')
        if not right_file.exists():
            continue
        left, right = left_file.read_bytes(), right_file.read_bytes()
        if len(left) != 480 * 272 * 4 or len(right) != len(left):
            raise ValueError('Unexpected screen capture dimensions')
        gap = bytes([0, 0, 0, 255]) * 16
        paired = b''.join(left[y * 1920:(y + 1) * 1920] + gap + right[y * 1920:(y + 1) * 1920] for y in range(272))
        path = left_file.with_name('screens.png')
        write_png(path, paired, 976, 272)
        print(path)


if __name__ == '__main__':
    main()
