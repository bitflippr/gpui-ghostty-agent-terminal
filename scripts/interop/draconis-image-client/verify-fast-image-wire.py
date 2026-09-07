"""Check a redirected iTerm2 multipart transfer preserves a source image exactly."""
import argparse
import base64
import hashlib
from pathlib import Path

START = b"\x1b]1337;MultipartFile="
PART = b"\x1b]1337;FilePart="
END = b"\x1b]1337;FileEnd\x07"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify(wire, original):
    require(wire.count(START) == 1, "Expected one multipart header")
    require(wire.count(END) == 1, "Expected one FileEnd")
    start = wire.index(START) + len(START)
    header_end = wire.find(b"\x07", start)
    require(header_end >= 0, "Unterminated multipart header")
    attributes = {}
    for field in wire[start:header_end].split(b";"):
        key, separator, value = field.partition(b"=")
        require(separator and key not in attributes, "Invalid or duplicate header attribute")
        attributes[key] = value
    require(attributes.get(b"inline") == b"1", "Expected inline image transfer")
    require(attributes.get(b"size") == str(len(original)).encode(), "Advertised file size differs from source")

    parts = []
    cursor = header_end + 1
    while wire.startswith(PART, cursor):
        payload_start = cursor + len(PART)
        part_end = wire.find(b"\x07", payload_start)
        require(part_end >= 0, "Unterminated FilePart")
        require(part_end + 1 - cursor <= 65536, "Multipart framing exceeds 64 KiB")
        payload = wire[payload_start:part_end]
        require(payload and len(payload) % 4 == 0, "Invalid FilePart base64 length")
        parts.append(payload)
        cursor = part_end + 1
    require(parts, "No multipart data found")
    require(wire.startswith(END, cursor), "FileEnd must immediately follow the FileParts")
    require(wire.count(PART) == len(parts), "FilePart found outside the multipart transfer")
    payload = base64.b64decode(b"".join(parts), validate=True)
    require(payload == original, "Transferred file differs from source")
    return len(parts), payload


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("wire", type=Path)
    parser.add_argument("image", type=Path)
    args = parser.parse_args()
    try:
        count, payload = verify(args.wire.read_bytes(), args.image.read_bytes())
    except (OSError, ValueError) as error:
        parser.exit(1, f"FAIL: {error}\n")
    print(f"PASS {count} parts, {len(payload)} unchanged bytes, SHA256={hashlib.sha256(payload).hexdigest()}")


if __name__ == "__main__":
    main()
