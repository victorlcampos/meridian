#!/usr/bin/env python3
"""Regenerate the data files embedded in the binary (assets/).

Sources, downloaded on demand and cached in tools/.cache/:
  * GeoNames (CC BY 4.0): cities15000, countryInfo, admin1CodesASCII
  * Natural Earth (public domain): 1:50m land and lakes polygons

Outputs:
  assets/cities.tsv     one city per line, most populous first:
                        name, country code, admin1 name, lat, lon, IANA zone,
                        population, alternate names ("|"-separated)
  assets/countries.tsv  country code, country name
  assets/land.pbm       equirectangular land mask, P4 bitmap, 1 = land

Usage: python3 tools/gen_assets.py
"""

import io
import json
import math
import os
import struct
import subprocess
import sys
import unicodedata
import zipfile
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE = os.path.join(ROOT, "tools", ".cache")
ASSETS = os.path.join(ROOT, "assets")

GEONAMES = "https://download.geonames.org/export/dump/"
NATURAL_EARTH = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/"

MASK_WIDTH, MASK_HEIGHT = 1440, 720

# Alternate names ("Londres", "Tóquio", "Nova Iorque") only for cities this big,
# to keep the embedded file small.
ALT_NAMES_MIN_POPULATION = 250_000
MAX_ALT_NAMES = 100

# Historical, abandoned or destroyed places are not useful for a clock.
EXCLUDED_FEATURE_CODES = {"PPLH", "PPLQ", "PPLW", "PPLCH"}


def fetch(base, name):
    # curl rather than urllib: download.geonames.org resets urllib connections.
    os.makedirs(CACHE, exist_ok=True)
    dest = os.path.join(CACHE, name)
    if not os.path.exists(dest):
        print(f"downloading {base}{name}", file=sys.stderr)
        subprocess.run(["curl", "-sSfL", "--max-time", "300", "-o", dest, base + name], check=True)
    return dest


def fold(text):
    text = unicodedata.normalize("NFKD", text)
    text = "".join(ch for ch in text if not unicodedata.combining(ch)).lower()
    text = "".join(ch if ch.isalnum() else " " for ch in text.replace("'", ""))
    return " ".join(text.split())


def is_latin(text):
    for ch in text:
        if ch.isalpha():
            try:
                if not unicodedata.name(ch).startswith("LATIN"):
                    return False
            except ValueError:
                return False
    return True


def clean(text):
    return " ".join(text.replace("|", " ").split())


def build_cities():
    countries = {}
    with open(fetch(GEONAMES, "countryInfo.txt"), encoding="utf-8") as f:
        for line in f:
            if line.startswith("#"):
                continue
            cols = line.rstrip("\n").split("\t")
            countries[cols[0]] = clean(cols[4])

    admin1 = {}
    with open(fetch(GEONAMES, "admin1CodesASCII.txt"), encoding="utf-8") as f:
        for line in f:
            cols = line.rstrip("\n").split("\t")
            admin1[cols[0]] = clean(cols[1])

    rows = []
    with zipfile.ZipFile(fetch(GEONAMES, "cities15000.zip")) as z:
        with z.open("cities15000.txt") as raw:
            for line in io.TextIOWrapper(raw, encoding="utf-8"):
                cols = line.rstrip("\n").split("\t")
                name, alternates = clean(cols[1]), cols[3]
                lat, lon = float(cols[4]), float(cols[5])
                feature, cc, tz = cols[7], cols[8], cols[17]
                population = int(cols[14] or 0)
                if feature in EXCLUDED_FEATURE_CODES or not tz or cc not in countries:
                    continue

                alts = []
                if population >= ALT_NAMES_MIN_POPULATION and alternates:
                    seen = {fold(name), fold(cols[2])}
                    for alt in alternates.split(","):
                        alt = clean(alt)
                        key = fold(alt)
                        # Lower-case entries are machine transliterations ("dong jing").
                        if len(key) < 2 or key in seen or not alt[0].isupper():
                            continue
                        if not is_latin(alt) or any(ch.isdigit() for ch in alt):
                            continue
                        seen.add(key)
                        alts.append(alt)
                        if len(alts) == MAX_ALT_NAMES:
                            break

                region = admin1.get(f"{cc}.{cols[10]}", "")
                rows.append((population, name, cc, region, lat, lon, tz, alts))

    rows.sort(key=lambda r: (-r[0], r[1]))
    with open(os.path.join(ASSETS, "cities.tsv"), "w", encoding="utf-8", newline="\n") as out:
        for population, name, cc, region, lat, lon, tz, alts in rows:
            out.write(f"{name}\t{cc}\t{region}\t{lat:.3f}\t{lon:.3f}\t{tz}\t{population}\t{'|'.join(alts)}\n")

    with open(os.path.join(ASSETS, "countries.tsv"), "w", encoding="utf-8", newline="\n") as out:
        for cc in sorted(countries):
            out.write(f"{cc}\t{countries[cc]}\n")

    print(f"cities: {len(rows)}, countries: {len(countries)}", file=sys.stderr)


def load_polygons(name):
    with open(fetch(NATURAL_EARTH, name), encoding="utf-8") as f:
        collection = json.load(f)
    polygons = []
    for feature in collection["features"]:
        geometry = feature["geometry"]
        if geometry is None:
            continue
        if geometry["type"] == "Polygon":
            polygons.append(geometry["coordinates"])
        elif geometry["type"] == "MultiPolygon":
            polygons.extend(geometry["coordinates"])
    return polygons


def rasterize(polygons, width, height):
    """Scanline fill (even-odd per polygon) sampled at pixel centers."""
    dx, dy = 360.0 / width, 180.0 / height
    crossings = [dict() for _ in range(height)]
    for index, rings in enumerate(polygons):
        for ring in rings:
            for (x1, y1), (x2, y2) in zip(ring, ring[1:]):
                if y1 == y2:
                    continue
                lo, hi = min(y1, y2), max(y1, y2)
                first = max(0, math.floor((90 - hi) / dy - 0.5))
                last = min(height - 1, math.floor((90 - lo) / dy - 0.5))
                for row in range(first, last + 1):
                    lat = 90 - (row + 0.5) * dy
                    if lo <= lat < hi:
                        lon = x1 + (lat - y1) * (x2 - x1) / (y2 - y1)
                        crossings[row].setdefault(index, []).append(lon)

    bitmap = [bytearray(width) for _ in range(height)]
    for row in range(height):
        for xs in crossings[row].values():
            xs.sort()
            for a, b in zip(xs[0::2], xs[1::2]):
                first = max(0, math.ceil((a + 180) / dx - 0.5))
                last = min(width - 1, math.ceil((b + 180) / dx - 0.5) - 1)
                for col in range(first, last + 1):
                    bitmap[row][col] = 1
    return bitmap


def write_pbm(path, bitmap, width, height):
    with open(path, "wb") as out:
        out.write(f"P4\n{width} {height}\n".encode())
        for row in bitmap:
            packed = bytearray((width + 7) // 8)
            for col, value in enumerate(row):
                if value:
                    packed[col >> 3] |= 0x80 >> (col & 7)
            out.write(packed)


def write_preview_png(path, bitmap, width, height):
    """Grayscale preview for humans (not embedded): land dark, water light."""
    raw = b"".join(b"\x00" + bytes(40 if v else 230 for v in row) for row in bitmap)

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", width, height, 8, 0, 0, 0, 0)
    with open(path, "wb") as out:
        out.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def build_land_mask():
    land = rasterize(load_polygons("ne_50m_land.geojson"), MASK_WIDTH, MASK_HEIGHT)
    lakes = rasterize(load_polygons("ne_50m_lakes.geojson"), MASK_WIDTH, MASK_HEIGHT)
    for row in range(MASK_HEIGHT):
        for col in range(MASK_WIDTH):
            if lakes[row][col]:
                land[row][col] = 0
    write_pbm(os.path.join(ASSETS, "land.pbm"), land, MASK_WIDTH, MASK_HEIGHT)
    write_preview_png(os.path.join(CACHE, "land_preview.png"), land, MASK_WIDTH, MASK_HEIGHT)
    filled = sum(sum(row) for row in land)
    print(f"land mask: {MASK_WIDTH}x{MASK_HEIGHT}, {filled / (MASK_WIDTH * MASK_HEIGHT):.1%} land", file=sys.stderr)


if __name__ == "__main__":
    os.makedirs(ASSETS, exist_ok=True)
    build_cities()
    build_land_mask()
