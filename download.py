#!/usr/bin/env python

import argparse
import yaml

from yt_dlp import YoutubeDL

parser = argparse.ArgumentParser()
parser.add_argument(
    "--path",
    default=".",
    help="Directory path to download videos to",
)
parser.add_argument(
    "--list",
    default="list.yaml",
    help="Path to list.yaml file",
)
parser.add_argument(
    "-q",
    "--quiet",
    default=False,
    action=argparse.BooleanOptionalAction,
    help="Quiet mode for yt-dlp",
)
parser.add_argument(
    "--progress",
    default=True,
    action=argparse.BooleanOptionalAction,
    help="Print progress for yt-dlp",
)
parser.add_argument(
    "--channel",
    default="",
    help="Download single channel",
)
args = parser.parse_args()

with open(args.list, "r", encoding="utf8") as stream:
    data = yaml.safe_load(stream)

for channel in data:
    name = channel["name"]
    url = channel["url"]

    if args.channel != "" and args.channel != name:
        continue

    ydl_opts = {
        "outtmpl": {
            "default": f"{args.path}/{name}/%(title)s [%(id)s].%(ext)s"
        },
        "writedescription": True,
        "writeinfojson": True,
        "writeannotations": True,
        "writethumbnail": True,
        "writesubtitles": True,
        "writeautomaticsub": True,
        "download_archive": f"{args.path}/archive",
        "progress": args.progress,
        "quiet": args.quiet,
    }
    with YoutubeDL(ydl_opts) as ydl:
        print(f"Downloading: {name} ({url})")
        try:
            ydl.download(url)
        except Exception:
            continue
