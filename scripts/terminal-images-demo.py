"""Visual image-protocol fixture; run inside Agent Terminal with Python 3."""
import base64
import struct
import sys
import zlib


def emit(value):
    sys.stdout.write(value)
    sys.stdout.flush()


def kitty(control, data=b""):
    emit("\x1b_G" + control + ";" + base64.b64encode(data).decode() + "\x1b\\")


def png(width, height, pixels):
    def chunk(kind, value):
        return struct.pack(">I", len(value)) + kind + value + struct.pack(">I", zlib.crc32(kind + value))
    rows = b"".join(b"\0" + pixels[y * width * 4:(y + 1) * width * 4] for y in range(height))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")


emit("\x1b[2J\x1b[HKitty + iTerm2 native rendering\r\n")
rgba = bytes(component for y in range(64) for x in range(64)
             for component in (255 if x < 32 else 0, 200 if y < 32 else 0, 255 if x >= 32 else 0, 128 if y >= 32 else 255))
emit("\x1b[3;1HKitty RGBA / alpha       Cropped right half       iTerm2 PNG\x1b[4;1H")
kitty("a=T,f=32,s=64,v=64,i=501,c=16,r=6,C=1,q=2", rgba)
emit("\x1b[4;25H")
kitty("a=p,i=501,p=2,x=32,w=32,c=12,r=6,C=1,q=2")
emit("\x1b[4;49H\x1b]1337;File=inline=1;width=16;height=6;preserveAspectRatio=0:" + base64.b64encode(png(64, 64, rgba)).decode() + "\a")
emit("\x1b[12;1HKitty animation: red / blue (one second per frame)\x1b[13;1H")
kitty("a=T,f=24,s=1,v=1,i=502,c=16,r=4,C=1,q=2", bytes([255, 0, 0]))
kitty("a=f,f=24,s=1,v=1,i=502,z=1000,q=2", bytes([0, 0, 255]))
kitty("a=a,i=502,r=1,z=1000,s=3,q=2")
emit("\x1b[19;1HImages above should remain anchored when scrolling.\r\n")
