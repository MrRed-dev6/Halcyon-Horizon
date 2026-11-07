#!/usr/bin/env python3
"""Halcyon CI: the pinned Minecraft version from Mojang's public servers.

usage: fetch_mojang.py OUTDIR   (run in the repository root, where research-lock.json is)

OUTDIR is laid out like the official launcher's game directory (versions/, libraries/, assets/),
with the pinned server jar beside the client jar. Every file is checked against research-lock.json.
The files stay on the CI runner: only Horizon's own compiled exporter classes leave it.
"""
import hashlib
import json
import os
import sys
import urllib.request
from concurrent.futures import ThreadPoolExecutor

MANIFEST = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
ASSET_HOST = "https://resources.download.minecraft.net/"


def get(url):
    for attempt in range(4):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": "Halcyon-CI"})
            with urllib.request.urlopen(request, timeout=60) as response:
                return response.read()
        except Exception:
            if attempt == 3:
                raise


def sha1(data):
    return hashlib.sha1(data).hexdigest()


def fetch(url, path, want):
    if os.path.isfile(path):
        with open(path, "rb") as f:
            if sha1(f.read()) == want:
                return
    data = get(url)
    if sha1(data) != want:
        raise SystemExit(f"wrong checksum: {url}")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)


def applies_on_linux(lib):
    rules = lib.get("rules")
    if not rules:
        return True
    allowed = False
    for rule in rules:
        if "features" in rule:
            continue
        os_name = rule.get("os", {}).get("name")
        if os_name is None or os_name == "linux":
            allowed = rule["action"] == "allow"
    return allowed


def main():
    out = sys.argv[1]
    pins = json.load(open("research-lock.json"))["minecraft"]
    version = pins["version"]
    accepted = {pins.get("manifest_sha1", "")} | set(pins.get("accepted_manifest_sha1", []))
    accepted_index = set(pins.get("accepted_asset_index_sha1", []))

    manifest = json.loads(get(MANIFEST))
    entry = next(v for v in manifest["versions"] if v["id"] == version)
    version_bytes = get(entry["url"])
    if sha1(version_bytes) not in accepted:
        raise SystemExit(f"the version JSON {sha1(version_bytes)} is not one research-lock.json accepts: {sorted(accepted)}")
    version_dir = os.path.join(out, "versions", version)
    os.makedirs(version_dir, exist_ok=True)
    with open(os.path.join(version_dir, f"{version}.json"), "wb") as f:
        f.write(version_bytes)
    meta = json.loads(version_bytes)

    jobs = []
    client = meta["downloads"]["client"]
    if client["sha1"] != pins["client_sha1"]:
        raise SystemExit("the client jar is not the pinned one")
    jobs.append((client["url"], os.path.join(version_dir, f"{version}.jar"), client["sha1"]))
    server = meta["downloads"]["server"]
    if server["sha1"] != pins["server_sha1"]:
        raise SystemExit("the server jar is not the pinned one")
    jobs.append((server["url"], os.path.join(version_dir, f"{version}-server.jar"), server["sha1"]))

    for lib in meta["libraries"]:
        artifact = lib.get("downloads", {}).get("artifact")
        if artifact and applies_on_linux(lib):
            jobs.append((artifact["url"], os.path.join(out, "libraries", artifact["path"]), artifact["sha1"]))

    index = meta["assetIndex"]
    index_bytes = get(index["url"])
    if sha1(index_bytes) not in accepted_index:
        raise SystemExit("the asset index is not one research-lock.json accepts")
    index_path = os.path.join(out, "assets", "indexes", index["id"] + ".json")
    os.makedirs(os.path.dirname(index_path), exist_ok=True)
    with open(index_path, "wb") as f:
        f.write(index_bytes)
    for o in {v["hash"] for v in json.loads(index_bytes)["objects"].values()}:
        jobs.append((f"{ASSET_HOST}{o[:2]}/{o}", os.path.join(out, "assets", "objects", o[:2], o), o))

    print(f"Minecraft {version}: {len(jobs)} files")
    with ThreadPoolExecutor(16) as pool:
        list(pool.map(lambda job: fetch(*job), jobs))
    print("done")


main()
