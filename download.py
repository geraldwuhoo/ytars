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
        # Download in priority:
        # 1. webm compatible: 4320p > 2160p > 1440p > 1080p > 720p, AV1 > VP9, HFR preferred, OPUS
        # 2. mp4 compatible: 1080p and lower, H264, M4A
        # To avoid .mkv files (mainly h264 + opus), which cannot be played in the browser
        "format": '(571/402/272/701/401/315/313/700/400/308/271/699/399/303/248/698/398/302/247)+bestaudio[acodec=opus]/bestvideo[vcodec^=avc1]+bestaudio[ext=m4a]/bestvideo+bestaudio/best',
    }
    with YoutubeDL(ydl_opts) as ydl:
        print(f"Downloading: {name} ({url})")
        try:
            ydl.download(url)
        except Exception:
            continue
